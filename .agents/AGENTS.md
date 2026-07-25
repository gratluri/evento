# Evento Project Rules

## Validation and Testing
- **Production-Grade Validation**: For every incremental step we take, we must validate the code as if this is a production system. Assume nothing is "local only". All code paths, configurations, and environment assumptions must be production-ready and rigorously tested at each step before moving forward.
- **Enterprise-Grade Comprehensive Functionality**: Evento is being built as an enterprise-grade software to compete with established tools like JMeter. On every future feature, you MUST proactively ask me about comprehensive functionality, edge cases, failure states (e.g. what happens when a dependency goes offline?), and build in robust monitoring and graceful degradation by default.
