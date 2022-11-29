// These are set by build.rs
static GIT_REV: &str = env!("GIT_REV");
static GIT_DESCRIBE: &str = env!("GIT_DESCRIBE");
static GIT_DIRTY: &str = env!("GIT_DIRTY");
static BUILD_TIME: &str = env!("BUILD_TIME");
static PROFILE: &str = env!("PROFILE");

#[derive(Debug, Clone)]
pub struct Version {
    pub semversion: semver::Version,
    pub git_rev: String,
    pub git_is_dirty: bool,
    pub build_time: chrono::DateTime<chrono::FixedOffset>,
    pub build_profile: String,
}

impl Version {
    pub fn from_env() -> Self {
        // we set DITTO_TEST_VERSION for integration snapshot tests
        // (where version outputs need to be deterministic)
        if let Ok(_test) = std::env::var("DITTO_TEST_VERSION") {
            return Self::new_test();
        }
        Self {
            semversion: semver::Version::parse(GIT_DESCRIBE)
                .unwrap_or_else(|_| panic!("invalid GIT_DESCRIBE: \"{GIT_DESCRIBE}\"")),
            git_rev: GIT_REV.to_owned(),
            git_is_dirty: GIT_DIRTY == "yes", // see build.rs
            build_time: chrono::DateTime::parse_from_rfc3339(BUILD_TIME)
                .unwrap_or_else(|_| panic!("invalid BUILD_TIME: \"{BUILD_TIME}\"")),
            build_profile: PROFILE.to_owned(),
        }
    }
    pub fn render_short(&self) -> String {
        format!(
            "{version}{dirty}",
            version = self.semversion,
            dirty = if self.git_is_dirty { "*" } else { "" },
        )
    }
    pub fn render_long(&self) -> String {
        format!(
            "{version}{dirty} {profile}\nbuilt at: {build_time}",
            version = self.semversion,
            dirty = if self.git_is_dirty { "*" } else { "" },
            profile = self.build_profile,
            build_time = self.build_time.to_rfc3339()
        )
    }
    fn new_test() -> Self {
        use chrono::TimeZone;
        Self {
            semversion: semver::Version::new(0, 0, 0),
            git_rev: String::from("test"),
            git_is_dirty: false,
            build_time: chrono::Utc
                .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
                .unwrap()
                .into(),
            build_profile: String::from("test"),
        }
    }
}
