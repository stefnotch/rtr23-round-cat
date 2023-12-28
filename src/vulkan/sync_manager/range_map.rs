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
    pub fn new(max_range: InnerMap::Range) -> Self {
        Self {
            max_range,
            inner: FullRangeOpt::All(None),
        }
    }

    pub fn new_from(inner: InnerMap, max_range: InnerMap::Range) -> Self {
        Self {
            max_range,
            inner: FullRangeOpt::Granular(inner),
        }
    }

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
        Self::new(max_range)
    }

    fn get_all(&self, key: &Self::Range) -> impl Iterator<Item = (Self::Range, &Self::Value)> + '_ {
        if !key.is_valid() {
            panic!("Invalid range");
        }
        if is_empty_range(key) {
            return Box::new(std::iter::empty()) as Box<dyn Iterator<Item = _> + '_>;
        }
        match &self.inner {
            FullRangeOpt::All(None) => Box::new(std::iter::empty()),
            FullRangeOpt::All(Some(value)) => Box::new(std::iter::once((*key, value))),
            FullRangeOpt::Granular(inner) => Box::new(inner.get_all(key)),
        }
    }

    fn overwrite(
        &mut self,
        key: Self::Range,
        value: Self::Value,
    ) -> Vec<(Self::Range, Self::Value)> {
        if !key.is_valid() {
            panic!("Invalid range");
        }
        if is_empty_range(&key) {
            return vec![];
        }
        if self.is_max_range(&key) {
            let old = self.get_all(&key).map(|(k, v)| (k, v.clone())).collect();
            self.inner = FullRangeOpt::All(Some(value));
            return old;
        }
        match &mut self.inner {
            FullRangeOpt::All(v) => {
                let old = v
                    .take()
                    .map(|v| vec![(self.max_range, v)])
                    .unwrap_or_default();
                let mut inner = InnerMap::new_with_max_range(self.max_range.clone());
                inner.overwrite(key, value);
                self.inner = FullRangeOpt::Granular(inner);
                old
            }
            FullRangeOpt::Granular(inner) => inner.overwrite(key, value),
        }
    }

    fn cut(&mut self, key: Self::Range) -> Vec<(Self::Range, Self::Value)> {
        if !key.is_valid() {
            panic!("Invalid range");
        }
        if is_empty_range(&key) {
            return vec![];
        }
        let is_max_range = self.is_max_range(&key);
        match &mut self.inner {
            FullRangeOpt::All(v) => {
                let old = v
                    .take()
                    .map(|v| vec![(self.max_range, v)])
                    .unwrap_or_default();
                if is_max_range {
                    self.inner = FullRangeOpt::All(None);
                } else {
                    self.inner = FullRangeOpt::Granular(InnerMap::new_with_max_range(
                        self.max_range.clone(),
                    ));
                }
                old
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
}

impl<I, K, V> RangeMap<I, K, V> {
    pub fn new() -> Self {
        Self {
            inner: discrete_range_map::DiscreteRangeMap::new(),
        }
    }
}

impl<I, K, V> RangeMapLike for RangeMap<I, K, V>
where
    I: discrete_range_map::PointType,
    V: Clone,
    K: discrete_range_map::RangeType<I>,
{
    type Point = I;
    type Range = K;
    type Value = V;

    fn new_with_max_range(_max_range: Self::Range) -> Self {
        Self::new()
    }

    fn get_all(&self, key: &Self::Range) -> impl Iterator<Item = (Self::Range, &Self::Value)> + '_ {
        self.inner.overlapping(*key).map(|(k, v)| (*k, v))
    }

    fn overwrite(
        &mut self,
        key: Self::Range,
        value: Self::Value,
    ) -> Vec<(Self::Range, Self::Value)> {
        self.inner.insert_overwrite(key, value).collect()
    }

    fn cut(&mut self, key: Self::Range) -> Vec<(Self::Range, Self::Value)> {
        self.inner.cut(key).collect()
    }

    fn insert_if_empty(&mut self, key: Self::Range, value: Self::Value) -> Result<(), Self::Value> {
        self.inner.insert_strict(key, value).map_err(|v| v.value)
    }
}

impl<I, K, V> Default for RangeMap<I, K, V> {
    fn default() -> Self {
        Self::new()
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
        if !key.is_valid() {
            panic!("Invalid range");
        }
        if is_empty_range(&key) {
            return vec![];
        }
        let overwritten = self
            .get_all(&key)
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
        Self::new(max_range.end())
    }

    fn get_all(&self, key: &Self::Range) -> impl Iterator<Item = (Self::Range, &Self::Value)> + '_ {
        if !key.is_valid() {
            panic!("Invalid range");
        }
        // Filter subsequent duplicates
        self.values[RangeInclusive::from(*key)]
            .iter()
            .flatten()
            .enumerate()
            .scan((key.start(), None), |(start, prev), (index, next)| {
                if &Some(next) == prev {
                    Some(None)
                } else {
                    *start = index;
                    *prev = Some(next);
                    Some(Some((
                        InclusiveInterval::from((*start)..=(index + 1)),
                        next,
                    )))
                }
            })
            .flatten()
    }

    fn overwrite(
        &mut self,
        key: Self::Range,
        value: Self::Value,
    ) -> Vec<(Self::Range, Self::Value)> {
        self.overwrite_internal(key, Some(value))
    }

    fn cut(&mut self, key: Self::Range) -> Vec<(Self::Range, Self::Value)> {
        self.overwrite_internal(key, None)
    }
}

fn is_empty_range<R: discrete_range_map::RangeType<P>, P: discrete_range_map::PointType>(
    range: &R,
) -> bool {
    range.start() >= range.end()
}

/// A map from ranges to values.
pub trait RangeMapLike {
    type Point: discrete_range_map::PointType;
    type Range: discrete_range_map::RangeType<Self::Point>;
    type Value: Clone;

    fn new_with_max_range(max_range: Self::Range) -> Self;

    /// Returns all values that overlap with the given key.
    fn get_all(&self, key: &Self::Range) -> impl Iterator<Item = (Self::Range, &Self::Value)> + '_;

    /// Overwrites the values in the range, and returns all values that were overwritten.
    fn overwrite(
        &mut self,
        key: Self::Range,
        value: Self::Value,
    ) -> Vec<(Self::Range, Self::Value)>;

    /// Inserts the given value at the given key, if there is no value at that key.
    /// Otherwise, returns the user supplied value.
    fn insert_if_empty(&mut self, key: Self::Range, value: Self::Value) -> Result<(), Self::Value> {
        if self.get_all(&key).next().is_none() {
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
