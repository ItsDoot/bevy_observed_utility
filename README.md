# bevy_observed_utility

A state-of-the-art utility AI library for Bevy, built using ECS [observers](https://docs.rs/bevy/latest/bevy/ecs/prelude/struct.Observer.html), with a focus on ergonomics and correctness.

See the [documentation](https://docs.rs/bevy_observed_utility/latest/bevy_observed_utility) for a complete walkthrough example of using the library.

## Design Goals

In order of priority:

- **Correctness**
    - Scoring entity trees are scored in depth-first post-order traversal, ensuring that all children are scored before their parents.
- **Ergonomics**:
    - Adding scoring, picking, and actions to pre-existing entities should have little boilerplate.
- **Modularity**:
    - Adding new kinds of scoring and picking should be easy.
    - Adding different ways of handling actions should be easy.
    - Both turn-based and real-time games should be supported.
- **Performance**:
    - Pay only for what you use: Scoring and picking observers are only added if they are used.
    - Scoring and picking should be reasonably fast. Action performance is up to the user.

## Lifecycle Visualized

![lifecycle](https://raw.githubusercontent.com/ItsDoot/bevy_observed_utility/main/lifecycle.png)

## License

`bevy_observed_utility` is dual-licensed under either:

- MIT License
- Apache License, Version 2.0

at your option.