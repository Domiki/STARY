use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct LaunchGroup {
    pub role: Option<String>,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct LaunchContext {
    pub groups: Vec<LaunchGroup>,
    pub language: Option<String>,
}

const LANGUAGE_FLAG: &str = "--lang";
const ROLE_FLAG: &str = "--role";

fn is_flag(argument: &str) -> bool {
    argument.starts_with('-')
}

fn push_path(groups: &mut Vec<LaunchGroup>, path: String) {
    match groups.last_mut() {
        Some(group) => group.paths.push(path),
        None => groups.push(LaunchGroup {
            role: None,
            paths: vec![path],
        }),
    }
}

impl LaunchContext {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut arguments = args.into_iter();
        let mut context = Self::default();
        while let Some(argument) = arguments.next() {
            match argument.split_once('=') {
                Some((LANGUAGE_FLAG, code)) => context.language = Some(code.to_string()),
                Some((ROLE_FLAG, role)) => context.groups.push(group_for(role)),
                _ if argument == LANGUAGE_FLAG => context.language = arguments.next(),
                _ if argument == ROLE_FLAG => context
                    .groups
                    .push(group_for(&arguments.next().unwrap_or_default())),
                _ if is_flag(&argument) => {}
                _ => push_path(&mut context.groups, argument),
            }
        }
        context
    }
}

fn group_for(role: &str) -> LaunchGroup {
    LaunchGroup {
        role: Some(role.to_string()),
        paths: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> LaunchContext {
        LaunchContext::from_args(args.iter().map(|argument| argument.to_string()))
    }

    fn group(role: Option<&str>, paths: &[&str]) -> LaunchGroup {
        LaunchGroup {
            role: role.map(str::to_string),
            paths: paths.iter().map(|path| path.to_string()).collect(),
        }
    }

    #[test]
    fn collects_paths_and_language() {
        let context = parse(&["--lang", "en", "/a/one.fits", "/b/two.jpg"]);
        assert_eq!(context.language.as_deref(), Some("en"));
        assert_eq!(
            context.groups,
            vec![group(None, &["/a/one.fits", "/b/two.jpg"])]
        );
    }

    #[test]
    fn accepts_equals_form_and_ignores_unknown_flags() {
        let context = parse(&["--lang=ko", "-psn_0_123", "/a/one.fits"]);
        assert_eq!(context.language.as_deref(), Some("ko"));
        assert_eq!(context.groups, vec![group(None, &["/a/one.fits"])]);
    }

    #[test]
    fn role_flag_starts_a_new_group() {
        let context = parse(&[
            "/a/x.fits",
            "--role",
            "dark",
            "/d/1",
            "/d/2",
            "--role=flat",
            "/f/1",
        ]);
        assert_eq!(
            context.groups,
            vec![
                group(None, &["/a/x.fits"]),
                group(Some("dark"), &["/d/1", "/d/2"]),
                group(Some("flat"), &["/f/1"]),
            ]
        );
    }

    #[test]
    fn empty_args_give_default_context() {
        assert_eq!(parse(&[]), LaunchContext::default());
    }
}
