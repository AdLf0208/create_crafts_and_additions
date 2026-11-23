fn wire_render_needs(needs: &mut RenderNeeds) {
  // The wire's own pass tests against the world's depth, so it must be kept for this frame.
  // Several mods may ask; setting the flag leaves what others set alone.
  needs.world_depth = true;
}
