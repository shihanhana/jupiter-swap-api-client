use {
    serde::{de, Deserialize, Deserializer, Serialize, Serializer},
    std::{fmt::Display, str::FromStr},
};

pub fn serialize<T, S>(t: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer,
{
    match t {
        Some(t) => serializer.collect_str(t),
        None => serializer.serialize_none()
    }
}

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromStr,
    D: Deserializer<'de>,
    <T as FromStr>::Err: std::fmt::Debug,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) => s
            .parse()
            .map(Some)
            .map_err(|e| de::Error::custom(format!("Parse error: {:?}", e))),
        None => Ok(None),
    }
}
