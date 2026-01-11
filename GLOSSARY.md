# Glossary


### AIController
An Entity that can autonomously select and execute `Actions`.


### Pawn 
The actual thing(s) the AIController drives. 

This could be a single NPC, a squad of them, a crowd, a faction, 
or even the abstract 'game flow' for AI Directors à la Left 4 Dead. 


### Action 

A thing the AI can do - moving, spawning an entity, shooting, or picking up an item. 

Consists of an `ActionTemplate` and an `ActionContext`.


### ActionTemplate 

An abstract 'essence' of the Action, 
e.g. `Move` in `Move(somewhere)` or `PickUp` in `PickUp(something)`. 

Stores the data needed by the decision engine to turn it into an `Action`.


### ActionContext 

The 'target'/'object' of the Action. 
e.g. `(somewhere)` in `Move(somewhere)` or `(something)` in `PickUp(something)`. 

In Cranium, those are stored as Bevy Entities, i.e. lightweight identifiers that can be fed
into Queries to retrieve any and all Components they might have, cheaply (`O(1)` time!).


### ActionSet 

A collection of `ActionTemplates`. Simple as that.


### SmartObject 

A thing that provides one or more `ActionSet` to appropriate `AIControllers`. 

For example, a `Cake` item may be a `SmartObject` providing an `Eat()` `ActionTemplate` to the holder. 

The `Pawn` itself will often be a `SmartObject` providing various movement/combat/interaction `Actions`. 


### ContextFetcher (CF)

A function that returns `Contexts` for an `Action`. 

This can and likely usually will be an ECS `System` of some kind to allow for making ECS `Queries`. 

Should usually be generic to be reusable across disparate `Actions`, although specialized CFs 
can sometimes be handy, especially if optimization is important. 

Generally expected to be supplied by users to tailor the AI to a specific game.


### Consideration 

A function (Bevy `System`) that provides specific quantifiable data about the Context value. 

For example, given a `Heal(Friend)` Action, a Consideration may return the Friend's 
current `Hitpoints` or its euclidean distance to `Pawn`. 

Receives a standard set of metadata inputs (the requesting `AiController`, its `Pawn`, and the current 
`Context`, all as raw `Entity` IDs) to facilitate building and running Queries quickly and easily. 

Considerations are generally expected to be supplied by users to tailor the AI to a specific game. 

As long as your function satisfies the `Consideration` interface (output type, handle piped inputs, 
and read-only), you can do anything you want as part of the Consideration logic.

Considerations can be exposed to Cranium using the `.register_consideration(func, key)` method on 
Apps and Worlds.


### UtilityCurve 

An arbitrary function with inputs and outputs on a unit interval (i.e. 0.0-1.0). 

Used to shape the Utility response - is higher better or worse? How fast does it change? 


### Consideration Score

The `Consideration`'s output after rescaling to a unit interval (i.e. 0.0-1.0) between 
its associated `Min` and `Max` values (as defined on the ActionTemplate) and applying 
the associated `UtilityCurve`.


### Priority

A multiplier expressing the relative 'urgency' of Actions. 

For example, healing a bleeding wound should always win over taking a nap, 
even if both Action's unmultiplied Action Scores were at maximum (1.0). 


### Action Score

The product of all `Consideration Scores` for an Action, starting at 1.0, 
followed by an num-Considerations-adjustment, and multiplying by `Priority` of the Action.

For example, if we have an `Action` with two `Considerations`, both of which returned 0.5, 
and a `Priority` of 10.0, then the final score will be `10. * (0.5 * 0.5)` => `2.5`.

(This is a lie - it will be *very slightly* higher because of the adjustment, 
but that's a bit too complicated for a quick explanation).

This is the score that ultimately decides whether an Action will be picked or not - highest wins.


### ActionTracker

An optional Entity providing a range of (even more optional) services to assist 
with managing the execution of the selected Action.

This may include tracking metadata about its creation and last update time, 
providing a 'tick-me' marker if you want to use an `Update()`-style loop for 
implementing your Action, a timer providing timedeltas since last tick, and more.

The default set of services provided can be customized by modifying 
the config stored in the `UserDefaultActionTrackerSpawnConfig` Resource.

This is effectively 'tooling sugar' to make it easier to build your Actions.
