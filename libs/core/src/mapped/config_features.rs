//! Config (metadata-only) feature views over the mapped file.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::Result;
use crate::format::{CompiledConfigFeature, invalid_compiled, read_string_range_at, read_u32_at};

pub struct ConfigFeatureView<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    metadata_count: usize,
    payload_start: usize,
}

impl<'a> ConfigFeatureView<'a> {
    pub(crate) fn new(
        path: &'a Path,
        bytes: &'a [u8],
        feature: &CompiledConfigFeature,
    ) -> Result<Self> {
        let mut offset = feature.payload_start;
        let metadata_count = read_u32_at(path, bytes, &mut offset)? as usize;
        if metadata_count != feature.metadata_count {
            return Err(invalid_compiled(
                path,
                "config metadata count does not match metadata",
            ));
        }
        Ok(Self {
            path,
            bytes,
            metadata_count,
            payload_start: feature.payload_start,
        })
    }

    pub fn get(&self, expected_key: &str) -> Result<Option<Option<&'a str>>> {
        for row in self.items() {
            let (key, value) = row?;
            if key == expected_key {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    pub fn metadata_value(&self, key: &str) -> Result<Option<&'a str>> {
        let value = self.get(key)?.flatten();
        if value.is_some() || key != "valueType" {
            return Ok(value);
        }
        Ok(self.get("value_type")?.flatten())
    }

    pub fn metadata(&self) -> Result<BTreeMap<String, Option<String>>> {
        let mut metadata = BTreeMap::new();
        for row in self.items() {
            let (key, value) = row?;
            metadata.insert(key.to_string(), value.map(str::to_string));
        }
        Ok(metadata)
    }

    pub fn items(&self) -> ConfigMetadataItems<'a> {
        let offset = self.payload_start + 4;
        ConfigMetadataItems {
            path: self.path,
            bytes: self.bytes,
            offset,
            remaining: self.metadata_count,
        }
    }
}

pub struct ConfigMetadataItems<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
    remaining: usize,
}

impl<'a> Iterator for ConfigMetadataItems<'a> {
    type Item = Result<(&'a str, Option<&'a str>)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;

        let key_range = match read_string_range_at(self.path, self.bytes, &mut self.offset) {
            Ok(range) => range,
            Err(error) => return Some(Err(error)),
        };
        let key = match std::str::from_utf8(&self.bytes[key_range]) {
            Ok(key) => key,
            Err(_) => return Some(Err(invalid_compiled(self.path, "invalid utf-8 string"))),
        };
        let tag = match self.bytes.get(self.offset) {
            Some(tag) => *tag,
            None => {
                return Some(Err(invalid_compiled(
                    self.path,
                    "unexpected end of config metadata value tag",
                )));
            }
        };
        self.offset += 1;
        match tag {
            0 => Some(Ok((key, None))),
            1 => {
                let value_range =
                    match read_string_range_at(self.path, self.bytes, &mut self.offset) {
                        Ok(range) => range,
                        Err(error) => return Some(Err(error)),
                    };
                Some(
                    std::str::from_utf8(&self.bytes[value_range])
                        .map(|value| (key, Some(value)))
                        .map_err(|_| invalid_compiled(self.path, "invalid utf-8 string")),
                )
            }
            _ => Some(Err(invalid_compiled(
                self.path,
                "invalid config metadata value tag",
            ))),
        }
    }
}
