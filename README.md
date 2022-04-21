# About

A lock using thread parking and a queue.

Not as unfair as the [naive spin lock](https://github.com/PoorlyDefinedBehaviour/spinlock)

## Comparisong with naive spin lock

- Wastes less cpu cycles
- Kinda fair

## Don't want to use atomics?

Check out [Peterson's algorithm](https://github.com/PoorlyDefinedBehaviour/petersons_mutual_exclusion_algorithm)
