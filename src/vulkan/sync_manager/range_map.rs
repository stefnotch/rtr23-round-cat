use std::ops::RangeInclusive;

use discrete_range_map::{InclusiveInterval, InclusiveRange};

/// Optimized version of `RangeMapLike` for the case where the range is fully covered by the value.
pub struct OptRangeMap<InnerMap, Value, Range> {
    max_range: Range,
    inner: FullRangeOpt<InnerMap, Value>,
}

enum FullRangeOpt<InnerMap, Value> {
    All(Option<Value>),
    Granular(InnerMap),
}

impl<InnerMap, I, K, V> OptRangeMap<InnerMap, V, K>
where
    InnerMap: RangeMapLike<Point = I, RangeKey = K, Value = V>,
    I: discrete_range_map::PointType,
    K: InclusiveRange<I>,
{
    pub fn new(max_range: K) -> Self {
        Self {
            max_range,
            inner: FullRangeOpt::All(None),
        }
    }

    pub fn new_from(inner: InnerMap, max_range: K) -> Self {
        Self {
            max_range,
            inner: FullRangeOpt::Granular(inner),
        }
    }

    pub fn is_max_range(&self, range: &K) -> bool {
        range.contains(self.max_range.start()) && range.contains(self.max_range.end())
    }
}

impl<InnerMap, I, K, V> RangeMapLike for OptRangeMap<InnerMap, V, K>
where
    InnerMap: RangeMapLike<Point = I, RangeKey = K, Value = V> + Default,
    I: discrete_range_map::PointType,
    K: discrete_range_map::RangeType<I>,
    V: Clone,
{
    type Point = I;
    type RangeKey = K;
    type Value = V;

    fn get_all(&self, key: &Self::RangeKey) -> impl Iterator<Item = Self::Value> + '_ {
        if !key.is_valid() {
            panic!("Invalid range");
        }
        if is_empty_range(key) {
            return Box::new(std::iter::empty()) as Box<dyn Iterator<Item = _> + '_>;
        }
        match &self.inner {
            FullRangeOpt::All(None) => Box::new(std::iter::empty()),
            FullRangeOpt::All(Some(value)) => Box::new(std::iter::once(value.clone())),
            FullRangeOpt::Granular(inner) => Box::new(inner.get_all(key)),
        }
    }

    fn overwrite(&mut self, key: Self::RangeKey, value: Self::Value) -> Vec<Self::Value> {
        if !key.is_valid() {
            panic!("Invalid range");
        }
        if is_empty_range(&key) {
            return vec![];
        }
        if self.is_max_range(&key) {
            let old = self.get_all(&key).collect();
            self.inner = FullRangeOpt::All(Some(value));
            return old;
        }
        match &mut self.inner {
            FullRangeOpt::All(v) => {
                let old = v.take().map(|v| vec![v]).unwrap_or_default();
                let mut inner = InnerMap::default();
                inner.overwrite(key, value);
                self.inner = FullRangeOpt::Granular(inner);
                old
            }
            FullRangeOpt::Granular(inner) => inner.overwrite(key, value),
        }
    }

    fn cut(&mut self, key: Self::RangeKey) -> Vec<Self::Value> {
        if !key.is_valid() {
            panic!("Invalid range");
        }
        if is_empty_range(&key) {
            return vec![];
        }
        let is_max_range = self.is_max_range(&key);
        match &mut self.inner {
            FullRangeOpt::All(v) => {
                let old = v.take().map(|v| vec![v]).unwrap_or_default();
                if is_max_range {
                    self.inner = FullRangeOpt::All(None);
                } else {
                    self.inner = FullRangeOpt::Granular(InnerMap::default());
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
    type RangeKey = K;
    type Value = V;

    fn get_all(&self, key: &Self::RangeKey) -> impl Iterator<Item = Self::Value> + '_ {
        self.inner.overlapping(*key).map(|(_, v)| v.clone())
    }

    fn overwrite(&mut self, key: Self::RangeKey, value: Self::Value) -> Vec<Self::Value> {
        self.inner
            .insert_overwrite(key, value)
            .map(|(_, v)| v)
            .collect()
    }

    fn cut(&mut self, key: Self::RangeKey) -> Vec<Self::Value> {
        self.inner.cut(key).map(|(_, v)| v).collect()
    }

    fn insert_if_empty(
        &mut self,
        key: Self::RangeKey,
        value: Self::Value,
    ) -> Result<(), Self::Value> {
        self.inner.insert_strict(key, value).map_err(|v| v.value)
    }
}

impl<I, K, V> Default for RangeMap<I, K, V> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SmallArrayRangeMap<V, const size: usize> {
    values: Box<[V]>,
}

impl<V, const size: usize> SmallArrayRangeMap<V, size> {
    pub fn new(value: V) -> Self
    where
        V: Copy,
    {
        Self {
            values: Box::new([value; size]),
        }
    }
}

impl<V, const size: usize> RangeMapLike for SmallArrayRangeMap<V, size>
where
    V: Copy + PartialEq,
{
    type Point = usize;
    type RangeKey = InclusiveInterval<usize>;
    type Value = V;

    fn get_all(&self, key: &Self::RangeKey) -> impl Iterator<Item = Self::Value> + '_ {
        if !key.is_valid() {
            panic!("Invalid range");
        }
        // Filter subsequent duplicates
        self.values[RangeInclusive::from(*key)]
            .iter()
            .scan(None, |prev, next| {
                if &Some(next) == prev {
                    Some(None)
                } else {
                    *prev = Some(next);
                    Some(Some(next))
                }
            })
            .filter_map(|x| x.copied())
    }

    fn overwrite(&mut self, key: Self::RangeKey, value: Self::Value) -> Vec<Self::Value> {
        if !key.is_valid() {
            panic!("Invalid range");
        }
        if is_empty_range(&key) {
            return vec![];
        }
        let overwritten = self.get_all(&key).collect::<Vec<_>>();
        for i in RangeInclusive::from(key) {
            self.values[i] = value;
        }
        overwritten
    }

    fn insert_if_empty(
        &mut self,
        _key: Self::RangeKey,
        _value: Self::Value,
    ) -> Result<(), Self::Value> {
        panic!("Not implemented")
    }

    fn cut(&mut self, _key: Self::RangeKey) -> Vec<Self::Value> {
        panic!("Not implemented")
    }
}

impl<V, const size: usize> Default for SmallArrayRangeMap<V, size>
where
    V: Copy + Default,
{
    fn default() -> Self {
        Self::new(V::default())
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
    type RangeKey: discrete_range_map::RangeType<Self::Point>;
    type Value: Clone;

    /// Returns all values that overlap with the given key.
    fn get_all(&self, key: &Self::RangeKey) -> impl Iterator<Item = Self::Value> + '_;

    /// Overwrites the values in the range, and returns all values that were overwritten.
    fn overwrite(&mut self, key: Self::RangeKey, value: Self::Value) -> Vec<Self::Value>;

    /// Inserts the given value at the given key, if there is no value at that key.
    /// Otherwise, returns the user supplied value.
    fn insert_if_empty(
        &mut self,
        key: Self::RangeKey,
        value: Self::Value,
    ) -> Result<(), Self::Value> {
        if self.get_all(&key).next().is_none() {
            self.overwrite(key, value);
            Ok(())
        } else {
            Err(value)
        }
    }

    /// Removes all values in the range, and returns them.
    /// Will split values that overlap with the range.
    fn cut(&mut self, key: Self::RangeKey) -> Vec<Self::Value>;
}
