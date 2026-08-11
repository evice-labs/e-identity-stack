use crate::state::ForumInstance;

pub fn process_verify_post(
    forum: &mut ForumInstance,
    _registry_root: [u8; 32],
    tracing_tag: [u8; 32],
) -> Result<(), &'static str> {
    if forum.used_tracing_tags.contains(&tracing_tag) {
        return Err("Post verification failed: Tracing tag already used (replay detected).");
    }

    forum.used_tracing_tags.push(tracing_tag);
    Ok(())
}
