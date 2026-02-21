//! Keypath parsing for dotted pack addressing
//!
//! Keypaths allow addressing packs with dot notation:
//!   - "openai"                → pack name (project/env from config)
//!   - "openai.new"            → pack name with variant (project/env from config)
//!   - "gzback.prod.openai"    → project.env.pack
//!   - "gzback.prod.openai.new" → project.env.pack.variant

use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct Keypath {
    pub project: String,
    pub environment: String,
    pub pack: String,
}

/// Parse a keypath string into project, environment, and pack name.
///
/// Resolution strategy:
/// - If project+env are provided (from config or CLI flags), the entire input is the pack name
/// - If only project is provided, first segment of input is env, rest is pack name
/// - If neither is provided, first two segments are project.env, rest is pack name
pub fn parse_keypath(
    input: &str,
    config_project: Option<&str>,
    config_env: Option<&str>,
) -> Result<Keypath> {
    if input.is_empty() {
        anyhow::bail!("Pack name cannot be empty");
    }

    match (config_project, config_env) {
        (Some(project), Some(env)) => Ok(Keypath {
            project: project.to_string(),
            environment: env.to_string(),
            pack: input.to_string(),
        }),
        (Some(project), None) => {
            let parts: Vec<&str> = input.splitn(2, '.').collect();
            if parts.len() < 2 {
                anyhow::bail!(
                    "No environment specified. Use -e/--environment or provide env.pack format"
                );
            }
            Ok(Keypath {
                project: project.to_string(),
                environment: parts[0].to_string(),
                pack: parts[1].to_string(),
            })
        }
        (None, _) => {
            let parts: Vec<&str> = input.splitn(3, '.').collect();
            if parts.len() < 3 {
                anyhow::bail!(
                    "No project/environment specified. Use -p/-e flags, \
                     create a .tinysecrets.toml, or provide project.env.pack format"
                );
            }
            Ok(Keypath {
                project: parts[0].to_string(),
                environment: parts[1].to_string(),
                pack: parts[2].to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_config_available() {
        let kp = parse_keypath("openai", Some("gzback"), Some("prod")).unwrap();
        assert_eq!(kp.project, "gzback");
        assert_eq!(kp.environment, "prod");
        assert_eq!(kp.pack, "openai");
    }

    #[test]
    fn test_variant_with_config() {
        let kp = parse_keypath("openai.new", Some("gzback"), Some("prod")).unwrap();
        assert_eq!(kp.pack, "openai.new");
    }

    #[test]
    fn test_full_keypath_no_config() {
        let kp = parse_keypath("gzback.prod.openai", None, None).unwrap();
        assert_eq!(kp.project, "gzback");
        assert_eq!(kp.environment, "prod");
        assert_eq!(kp.pack, "openai");
    }

    #[test]
    fn test_full_keypath_with_variant_no_config() {
        let kp = parse_keypath("gzback.prod.openai.new", None, None).unwrap();
        assert_eq!(kp.project, "gzback");
        assert_eq!(kp.environment, "prod");
        // splitn(3, '.') → ["gzback", "prod", "openai.new"]
        assert_eq!(kp.pack, "openai.new");
    }

    #[test]
    fn test_project_only_config() {
        let kp = parse_keypath("prod.openai", Some("gzback"), None).unwrap();
        assert_eq!(kp.project, "gzback");
        assert_eq!(kp.environment, "prod");
        assert_eq!(kp.pack, "openai");
    }

    #[test]
    fn test_project_only_config_with_variant() {
        let kp = parse_keypath("prod.openai.new", Some("gzback"), None).unwrap();
        assert_eq!(kp.project, "gzback");
        assert_eq!(kp.environment, "prod");
        assert_eq!(kp.pack, "openai.new");
    }

    #[test]
    fn test_empty_input_fails() {
        assert!(parse_keypath("", Some("gzback"), Some("prod")).is_err());
    }

    #[test]
    fn test_no_config_insufficient_segments() {
        assert!(parse_keypath("openai", None, None).is_err());
        assert!(parse_keypath("gzback.openai", None, None).is_err());
    }

    #[test]
    fn test_project_only_insufficient_segments() {
        assert!(parse_keypath("openai", Some("gzback"), None).is_err());
    }
}
