# Cortex

## Mission Statement: 

> See more vibrant, interesting, living and breathing virtual worlds in games  
> by making it as easy as possible for developers to create and run them efficiently.

`Cortex` is an opinionated, modular, data-driven (and data-oriented) Rust library 
for 'classical' AI (primarily but not exclusively Utility AI) for interactive applications 
such as games and simulations.

`Cortex` is built using the [Bevy Game Engine](https://bevyengine.org/). 

However, a key design goal of the library is to be usable *outside* of Bevy applications - even outside of Rust. 

In fact, a major motivator for its creation was the need for a performant AI engine 
for NPCs in a project written in a deeply locked-down, decades-old game engine. 


### Features

* **Data-driven** - Let coders code core capabilities and let designers (and modders!) play with them.
* **High-performance** - Beyond 'blazingly fast': parallelism, data-oriented processing, an efficient core AI paradigm, and a bag of game AI optimization tricks to keep framerates smooth and crowds large.
* **Batteries Included** - Covering the hard parts the tutorials skip. LODs, state machines, per-agent knowledge base and knowledge sharing... 
* **Modular** - Disable things you don't use! Add extra stuff you need from your own app code!
* **Generic** - For turn-based or real-time, strategy or shooters, or even a non-game app... except possibly chess engines. 
* **Proven** - Informed by over a decade of experience building complex and performant game AIs and a small forest's worth of papers about game AI design. 
* **Reactive** - Uses event-driven mechanisms to avoid paying for systems you don't use and allowing you to hook into the ones that you might be interested in in your own code.
* **Portable** - Use in your Bevy app as a Plugin or drive it yourself from another engine. 
* **Safe** - Typechecker-approved, uses thread-safe solutions where multithreading is relevant.


### Glossary

- `AIController` - An Entity that can autonomously select and execute `Actions`.
- `Pawn` - The actual thing(s) the AIController drives. A single NPC, a squad of them, a crowd, a faction, or even the abstract 'game flow' for AI Directors à la Left 4 Dead. 
- `Action` - A thing the AI can do - moving, spawning an entity, shooting, or picking up an item. Consists of an `ActionTemplate` and an `ActionContext`.
- `ActionTemplate` - An abstract 'essence' of the Action, e.g. Move(somewhere) or PickUp(something). Stores the data needed by the decision engine to turn it into an `Action`.
- `ActionContext` - The 'target'/'object' of the Action, e.g. a position to move to, an NPC to shoot at, or an item to pick up. The thing that goes in the 'somewhere'/'something' in the ActionTemplate.
- `ActionSet` - A collection of `ActionTemplates`.
- `SmartObject` - A thing that provides one or more `ActionSet` to appropriate `AIControllers`. For example, a Cake item may be a SmartObject providing an Eat() ActionTemplate to the holder. The Pawn itself will often be a SmartObject providing various movement/combat/interaction Actions. 
- `ContextFetcher` - A function that returns Contexts for an Action. This can and likely usually will be an ECS System of some kind to allow for making ECS World Queries. Should usually be generic to be reusable across disparate Action, although specialist ones are justifiable. Generally expected to be supplied by users to tailor the AI to a specific game.
- `Consideration` - A function that provides specific quantifiable data about the Context value. For example, given a Heal(Friend) Action, a Consideration may return the Friend's current Hitpoints or Distance to Pawn. An ECS System that receives a standard set of metadata inputs to facilitate Queries. Generally expected to be supplied by users to tailor the AI to a specific game.
- `UtilityCurve` - An arbitrary function with inputs and outputs on a unit interval (i.e. 0.0-1.0). Used to shape the Utility response - is higher better or worse? How fast does it change? 
- `Consideration Score` - The `Consideration`'s output after rescaling to a unit interval (i.e. 0.0-1.0) between its associated `Min` and `Max` values (as defined on the ActionTemplate) and applying the associated `UtilityCurve`.
- `Action Score` - The product of all `Consideration Scores` for an Action, starting at 1.0, followed by multiplying by `Priority` of the Action.
- `Priority` - A multiplier expressing the relative 'urgency' of Actions. Healing a bleeding wound should win over taking a nap even if both Action's unmultiplied Action Scores were at maximum (1.0). 
- `ActionTracker` - An optional Entity providing a range of (even more optional) services to assist with managing the execution of the selected Action.


### Architecture

The core `Cortex` engine largely follows the excellent `Infinite Axis Utility System` architecture as 
outlined in the classic GDC 2015 talk ["Building a Better Centaur: AI at Massive Scale"](https://gdcvault.com/play/1021848/Building-a-Better-Centaur-AI) by Mike Lewis and Dave Mark.

The basic idea is very simple: 
1) The AI (`AIController`) gathers all actions it can possibly take.
2) The AI calculates a normalized score for every candidate `Action`, fuzzy-logic style.
3) The AI simply picks the highest-scoring candidate.
4) The `Action` gets dispatched to the AI-controlled entity (a `Pawn`) for execution.

The `Glossary` section provides a quick run-through of the process, with terms
introduced roughly in the sequence in which they come up during the decision loop.

Of course, the devil is in the details. There are a couple of design decisions worth mentioning.

#### Data-driven

Recompiling is famously (especially in Rust) a great excuse to go get coffee, 
but it can quickly get tedious whenever you're taming an unruly AI. 

While reading the README of a Rust AI library suggests you're probably happy messing with 
code, a good number of AI designers, modders, and tooling developers would also be happier if
they didn't have to make code changes, and could instead tweak something in JSON or whatever.

`Cortex` is built around treating AI definitions as Assets to be loaded from files 
and mapped to an implementation at runtime using nice, human-readable string keys.

You CAN hardcode things instead of using data assets if you are concerned about exposing things 
to savvier users for whatever reason; a hybrid approach is also feasible.

#### SmartObjects-oriented

A key idea used to make agent capabilities manageable is **Smart Objects**, 
famously used across "The Sims" series as far back as 2000.

This is a decision borne out of the experiences of working with AIs retrofitted to operate 
in extremely semantically rich game-worlds - what do you do when even the *floor* can be 
interacted with given the right tools and you don't have infinite time to write all behaviors? 

By putting the definitions in world objects rather than in the AI and allowing AIs to 'gather' 
those from the game-world in appropriate contexts, we get all the features of traditional 
'dump it all in the NPC' approach where needed, plus a whole lot of flexibility and moddability 
for scenarios where the simple approach wouldn't cut it. 

#### AI-as-an-Engine

`Cortex` could not possibly provide support for every genre and use-case out of the box.

Even with open-source contributions, waiting for a release to support the use-cases 
that you need to build your applications would not be a good experience for library users.

However, the core concepts of the library are open for third-party code to extend by
registering implementations of Actions, Curves, Considerations, ContextFetchers, and more!

This covers both the game-specific logic you bring to the table when building with `Cortex`, 
but also leaves the door open for sub-libraries that provide useful AI tools and templates 
for a specific genre.


### Getting started

The two main patterns of using `Cortex` are: 
- **Native** - as a native part of your Bevy Engine applications.
- **AI Server** - as a separate "AI World" for non-Bevy and/or non-Rust applications.

The two approaches are more similar than they might appear. 

In both cases, we're running the same core runtime in an ECS World. 

The only difference is whether `Cortex` has direct access to your application data, and 
whether the AI loop is part of your application loop or 'manually driven' by your apps.

For the **Native** integration, `Cortex` provides a configurable Bevy Plugin which handles 
the majority of the basic setup for you - all you need to do is import it, add it to your 
app, and register any custom types for the library to use on your behalf in AI code.

For the **AI Server** integration, `Cortex` provides an API that lets you set up and drive 
a Bevy ECS World for the AI to operate in, and methods to update this world with data from 
your own application.

In either case, you will nearly always need to tailor three things to your needs:
1) ContextFetchers 
2) Considerations 
3) Actions


#### ContextFetchers 

`ContextFetchers` are Bevy Systems - for non-Bevy users, this means they are mostly straightforward 
Rust functions, except for being able to make Queries (fast lookups of Entities and their Components 
in the ECS World `Cortex` is running in).

`ContextFetchers` receive some pre-defined inputs (normal function parameters whose types are 
wrapped in Bevy's `In<T>` wrappers) to provide the necessary metadata to power your Queries.

`ContextFetchers` should be *strictly read-only* and should return 
lists of Entities that represent possible targets for Actions. 

For example, to eat an apple, a `ContextFetcher` for the `Eat` Action should return every 
Entity that has an IsApple Component (possibly within some radius, or un-owned, or any other 
extra logic you want the AI to account for - up to you).

You can think of a `ContextFetcher` as a simple **filter** - they trim down all the Entities 
in the World to just those that make sense for the Actions they work for.

Once you're done building your `ContextFetchers`, you can register them to the World easily 
using either the classic App Builder-style `app.register_context_fetcher(func, key)` or on 
the World directly using `world.register_context_fetcher(func, key)`. 

Rust's type system will stop you from registering a `ContextFetcher` if there is something 
wrong with the function you have built. 

If you cannot see the function, check your imports. 
If you have trouble with the key argument, you can just call 
`.into()` on your string key (since Cortex uses a wrapper type around raw string types).


#### Considerations 

`Considerations` are, like `ContextFetchers`, also Bevy Systems.
They also take predefined inputs to help you build Queries in them 
(including a Context returned from the `ContextFetcher`).

`Considerations` should look up some quantifiable data about the world and return it 
as a floating-point number. 

For example, if the `Context` is an enemy unit, we may return its Health, or its 
Distance to our Pawn, and do it as either raw numbers, or as percentage, or 
whatever - as long as it's a floating-point number, it's valid. 

Once you're done building your Consideration(s), you can register them easily using 
either the classic App Builder-style `app.register_consideration(func, key)` or on 
the World directly using `world.register_consideration(func, key)`. 

Rust's type system will stop you from registering a Consideration if there is something 
wrong with the function you have built. 

If you cannot see the function, check your imports. 
If you have trouble with the key argument, you can just call 
`.into()` on your string key (since Cortex uses a wrapper type around raw string types).


