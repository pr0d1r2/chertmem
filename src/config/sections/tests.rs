use super::Provider;
use crate::config::tests::{Scratch, Test, project_only, put};

#[test]
fn embedding_privacy_and_mining_keys_parse() -> Test {
    let text = "embedding.provider = \"openai\"\n\
        privacy.exclude = [\".env\", \"secrets/\"]\n\
        mining.enabled = false\nmining.min_occurrences = 5\n";
    let c = project_only("mining", text)?;
    assert_eq!(c.embedding.provider, Provider::Openai);
    assert_eq!(c.privacy.exclude, vec![".env", "secrets/"]);
    let m = c.mining;
    assert_eq!((m.enabled, m.min_occurrences), (false, 5));
    Ok(())
}

#[test]
fn user_file_alone_applies() -> Test {
    let s = Scratch::new("useronly")?;
    put(&s.user_file(), "embedding.provider = \"ollama\"\n")?;
    assert_eq!(s.load()?.config.embedding.provider, Provider::Ollama);
    Ok(())
}
