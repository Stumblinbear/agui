# Globals

`agui` includes some default application globals to standardize some common functionality. This includes `Mouse`, `Keyboard`, `Theme`, among other things we'll go over in the next few sections.

## Who implements these?

When building an integration for new renderer, including some of the systems necessary to update these globals is largely non-negotiable. `agui` has no way to know how to update `Mouse` or `Keyboard` state, as the core doesn't implement a windowing system, so this is left entirely to the integrations. It's unnecessary to go much deeper in this section, but will be covered later. Rest assured the functionality is standardized across integrations.