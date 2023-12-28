use std::ops::RangeInclusive;

use discrete_range_map::{InclusiveInterval, InclusiveRange};

/// Optimized version of `RangeMapLike` for the case where the range is fully covered by the value.
pub struct OptRangeMap<InnerMap>
where
    InnerMap: RangeMapLike,
{
    max_range: InnerMap::Range,
    inner: FullRangeOpt<InnerMap>,
}

enum FullRangeOpt<InnerMap>
where
    InnerMap: RangeMapLike,
{
    All(Option<InnerMap::Value>),
    Granular(InnerMap),
}

impl<InnerMap> OptRangeMap<InnerMap>
where
    InnerMap: RangeMapLike,
{
    pub fn is_max_range(&self, range: &InnerMap::Range) -> bool {
        range.contains(self.max_range.start()) && range.contains(self.max_range.end())
    }
}

impl<InnerMap> RangeMapLike for OptRangeMap<InnerMap>
where
    InnerMap: RangeMapLike,
{
    type Point = InnerMap::Point;
    type Range = InnerMap::Range;
    type Value = InnerMap::Value;

    fn new_with_max_range(max_range: Self::Range) -> Self {
        Self {
            max_range,
            inner: FullRangeOpt::All(None),
        }
    }

    fn max_range(&self) -> Self::Range {
        self.max_range
    }

    fn overlapping(
        &self,
        key: &Self::Range,
    ) -> impl Iterator<Item = (Self::Range, &Self::Value)> + '_ {
        self.assert_valid_range(key);
        if is_empty_range(key) {
            return Box::new(std::iter::empty()) as Box<dyn Iterator<Item = _> + '_>;
        }
        match &self.inner {
            FullRangeOpt::All(None) => Box::new(std::iter::empty()),
            FullRangeOpt::All(Some(value)) => Box::new(std::iter::once((self.max_range, value))),
            FullRangeOpt::Granular(inner) => Box::new(inner.overlapping(key)),
        }
    }

    fn overwrite(
        &mut self,
        key: Self::Range,
        value: Self::Value,
    ) -> Vec<(Self::Range, Self::Value)> {
        self.assert_valid_range(&key);
        if is_empty_range(&key) {
            return vec![];
        }
        if self.is_max_range(&key) {
            let old = self
                .overlapping(&key)
                .map(|(k, v)| (k, v.clone()))
                .collect();
            self.inner = FullRangeOpt::All(Some(value));
            return old;
        }
        match &mut self.inner {
            FullRangeOpt::All(old) => {
                let mut inner: InnerMap = InnerMap::new_with_max_range(self.max_range.clone());
                if let Some(old_value) = &old {
                    inner.overwrite(self.max_range.clone(), old_value.clone());
                }
                inner.overwrite(key, value);
                let result = old
                    .take()
                    .map(|v| vec![(key, v.clone())])
                    .unwrap_or_default();
                self.inner = FullRangeOpt::Granular(inner);
                result
            }
            FullRangeOpt::Granular(inner) => inner.overwrite(key, value),
        }
    }

    fn cut(&mut self, key: Self::Range) -> Vec<(Self::Range, Self::Value)> {
        self.assert_valid_range(&key);
        if is_empty_range(&key) {
            return vec![];
        }
        let is_max_range = self.is_max_range(&key);
        match &mut self.inner {
            FullRangeOpt::All(v) => {
                let old = v.take();
                if is_max_range {
                    self.inner = FullRangeOpt::All(None);
                } else {
                    let mut inner = InnerMap::new_with_max_range(self.max_range.clone());
                    if let Some(v) = &old {
                        inner.overwrite(self.max_range.clone(), v.clone());
                        inner.cut(key);
                    }
                    self.inner = FullRangeOpt::Granular(inner);
                }
                old.map(|v| vec![(self.max_range, v)]).unwrap_or_default()
            }
            FullRangeOpt::Granular(inner) => {
                let old = inner.cut(key);
                if is_max_range {
                    self.inner = FullRangeOpt::All(None);
                }
                old
            }
        }
    }
}

pub struct RangeMap<I, K, V> {
    inner: discrete_range_map::DiscreteRangeMap<I, K, V>,
    max_range: K,
}

impl<I, K, V> RangeMapLike for RangeMap<I, K, V>
where
    I: discrete_range_map::PointType,
    V: Clone,
    K: discrete_range_map::RangeType<I> + std::fmt::Debug,
{
    type Point = I;
    type Range = K;
    type Value = V;

    fn new_with_max_range(max_range: Self::Range) -> Self {
        Self {
            inner: discrete_range_map::DiscreteRangeMap::new(),
            max_range,
        }
    }

    fn max_range(&self) -> Self::Range {
        self.max_range
    }

    fn overlapping(
        &self,
        key: &Self::Range,
    ) -> impl Iterator<Item = (Self::Range, &Self::Value)> + '_ {
        self.assert_valid_range(key);
        self.inner.overlapping(*key).map(|(k, v)| (*k, v))
    }

    fn overwrite(
        &mut self,
        key: Self::Range,
        value: Self::Value,
    ) -> Vec<(Self::Range, Self::Value)> {
        self.assert_valid_range(&key);
        self.inner.insert_overwrite(key, value).collect()
    }

    fn cut(&mut self, key: Self::Range) -> Vec<(Self::Range, Self::Value)> {
        self.assert_valid_range(&key);
        self.inner.cut(key).collect()
    }

    fn insert_if_empty(&mut self, key: Self::Range, value: Self::Value) -> Result<(), Self::Value> {
        self.assert_valid_range(&key);
        self.inner.insert_strict(key, value).map_err(|v| v.value)
    }
}

pub struct SmallArrayRangeMap<V> {
    values: Vec<Option<V>>,
}

impl<V> SmallArrayRangeMap<V> {
    pub fn new(size: usize) -> Self
    where
        V: Copy,
    {
        Self {
            values: vec![None; size],
        }
    }

    fn overwrite_internal(
        &mut self,
        key: InclusiveInterval<usize>,
        value: Option<V>,
    ) -> Vec<(InclusiveInterval<usize>, V)>
    where
        V: Copy + PartialEq,
    {
        self.assert_valid_range(&key);
        if is_empty_range(&key) {
            return vec![];
        }
        let overwritten = self
            .overlapping(&key)
            .map(|(k, v)| (k, v.clone()))
            .collect::<Vec<_>>();
        for i in RangeInclusive::from(key) {
            self.values[i] = value;
        }
        overwritten
    }
}

impl<V> RangeMapLike for SmallArrayRangeMap<V>
where
    V: Copy + PartialEq,
{
    type Point = usize;
    type Range = InclusiveInterval<usize>;
    type Value = V;

    fn new_with_max_range(max_range: Self::Range) -> Self {
        assert!(max_range.is_valid());
        assert!(max_range.start() == 0);
        Self::new(max_range.end() + 1)
    }

    fn max_range(&self) -> Self::Range {
        InclusiveInterval::from(0..self.values.len())
    }

    fn overlapping(
        &self,
        key: &Self::Range,
    ) -> impl Iterator<Item = (Self::Range, &Self::Value)> + '_ {
        self.assert_valid_range(key);
        let start_index = key.start();
        // Filter subsequent duplicates
        self.values[RangeInclusive::from(*key)]
            .iter()
            .enumerate()
            .flat_map(move |(i, v)| match v {
                Some(v) => Some((i + start_index, v)),
                None => None,
            })
            .scan((start_index, None), |(start, prev), (index, next)| {
                if &Some(next) == prev {
                    Some(None)
                } else {
                    *start = index;
                    *prev = Some(next);
                    Some(Some((InclusiveInterval::from((*start)..=index), next)))
                }
            })
            .flatten()
    }

    fn overwrite(
        &mut self,
        key: Self::Range,
        value: Self::Value,
    ) -> Vec<(Self::Range, Self::Value)> {
        self.assert_valid_range(&key);
        self.overwrite_internal(key, Some(value))
    }

    fn cut(&mut self, key: Self::Range) -> Vec<(Self::Range, Self::Value)> {
        self.assert_valid_range(&key);
        self.overwrite_internal(key, None)
    }
}

fn is_empty_range<R: discrete_range_map::RangeType<P>, P: discrete_range_map::PointType>(
    range: &R,
) -> bool {
    range.start() > range.end()
}

/// A map from ranges to values.
pub trait RangeMapLike {
    type Point: discrete_range_map::PointType;
    type Range: discrete_range_map::RangeType<Self::Point> + std::fmt::Debug;
    type Value: Clone;

    fn new_with_max_range(max_range: Self::Range) -> Self;

    fn max_range(&self) -> Self::Range;

    fn assert_valid_range(&self, range: &Self::Range) {
        assert!(
            range.is_valid() && range.start() >= self.max_range().start(),
            //  Buffers can use vk::WHOLE_SIZE, which is larger than the max range
            // && range.end() <= self.max_range().end(),
            "Invalid range {:?} for max range {:?}",
            range,
            self.max_range()
        );
    }

    /// Returns all values that overlap with the given key.
    fn overlapping(
        &self,
        key: &Self::Range,
    ) -> impl Iterator<Item = (Self::Range, &Self::Value)> + '_;

    /// Overwrites the values in the range, and returns all values that were overwritten.
    /// Will split values that overlap with the range.
    fn overwrite(
        &mut self,
        key: Self::Range,
        value: Self::Value,
    ) -> Vec<(Self::Range, Self::Value)>;

    /// Inserts the given value at the given key, if there is no value at that key.
    /// Otherwise, returns the user supplied value.
    fn insert_if_empty(&mut self, key: Self::Range, value: Self::Value) -> Result<(), Self::Value> {
        self.assert_valid_range(&key);
        if self.overlapping(&key).next().is_none() {
            self.overwrite(key, value);
            Ok(())
        } else {
            Err(value)
        }
    }

    /// Removes all values in the range, and returns them.
    /// Will split values that overlap with the range.
    fn cut(&mut self, key: Self::Range) -> Vec<(Self::Range, Self::Value)>;
}
