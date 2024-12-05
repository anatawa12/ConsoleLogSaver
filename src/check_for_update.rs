use std::cmp::Ordering;
use std::str::FromStr;

#[derive(Eq, PartialEq)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    is_prerelease: bool,
}

impl Version {
    fn new(major: u32, minor: u32, patch: u32, is_prerelease: bool) -> Self {
        Self {
            major,
            minor,
            patch,
            is_prerelease,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| self.is_prerelease.cmp(&other.is_prerelease))
    }
}

impl FromStr for Version {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (numbers, pre) = s.split_once('-').unwrap_or((s, ""));
        let (major, rest) = numbers.split_once('.').ok_or(())?;
        let (minor, patch) = rest.split_once('.').ok_or(())?;
        let major = major.parse().map_err(|_| ())?;
        let minor = minor.parse().map_err(|_| ())?;
        let patch = patch.parse().map_err(|_| ())?;
        Ok(Version::new(major, minor, patch, !pre.is_empty()))
    }
}

const fn const_ref_unwrap<T, E>(result: &Result<T, E>) -> &T {
    match result {
        Ok(ok) => ok,
        Err(_) => panic!("unwrap failed"),
    }
}

static CURRENT_VERSION: Version = Version {
    major: *const_ref_unwrap(&u32::from_str_radix(env!("CARGO_PKG_VERSION_MAJOR"), 10)),
    minor: *const_ref_unwrap(&u32::from_str_radix(env!("CARGO_PKG_VERSION_MINOR"), 10)),
    patch: *const_ref_unwrap(&u32::from_str_radix(env!("CARGO_PKG_VERSION_PATCH"), 10)),
    is_prerelease: !env!("CARGO_PKG_VERSION_PRE").is_empty(),
};

static USER_AGENT: &str = concat!(
    "ConsoleLogSaver-update-checker/{",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/anatawa12/ConsoleLogSaver)"
);

pub fn check_for_update() -> Option<(bool, String)> {
    let response = ureq::get("https://github.com/anatawa12/ConsoleLogSaver/raw/master/latest.txt")
        .set("User-Agent", USER_AGENT)
        .call()
        .ok()?;
    let response = response.into_string().ok()?;
    let (latest, _) = response.split_once('\n').unwrap_or((&response, ""));
    let latest = latest.trim();
    let latest_parsed = Version::from_str(latest).ok()?;

    Some((latest_parsed > CURRENT_VERSION, latest.to_owned()))
}
