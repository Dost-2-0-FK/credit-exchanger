# credit-exchanger
Exchange service for blocs, zones and individuals

## Installation 

## Specification 

### Types

#### Users
There are three types of users: 
- Bloc
- Zone
- Individual
- Units

Each user has: 
- `unique_id: str`
- `credit: Credit`

Blocs and Zones have: 
- `resources: Dict<resource: str, credit: Credit>`

Individuals
- Do not have `resources`
- If an individual's `credit` reaches zero, a notice is sent to the `Blackout controller` (`BLACKOUT-SERVICE/api/credit-overflow?id=<unique_id>`)

Units 
- Do not have resources
- Never have incoming subscriptions (no "income" via subscriptions),
- Their `last_day_average` can be set via the API and is not calculated at the end of the day

#### Credits

General: 
- A credit can be "money" or "recourse"

Attributes: 
- `total: Float`
- `last_day_avrg: Float  # average hourly credit of the last day` 
- `subscriptions: List<Subscriptions>`
- `history: List<float>` 

Methods: 
- `hourly(incomming_subsctiptions: List<Float>) -> Float`
- `calc_avrg() -> None`
- `evaluate() -> None` 

Logic: 
- The `hourly(incoming: List<Float>)` function calculates: `sum(incoming) - sum([s.calc(last_day_average) for s in subscriptions]` and the result is stored to `history`.
- `last_day_avrg` is calculated at `00:00`: `last_day_avrg = avrg(history)`. (Not for Trusts) The `history` is emptied afterwards.
- The `evaluate()` function evaluates all subscriptions
- Subscriptions with higher priority are evaluated first
- Subscriptions with `type="sr"` are *always* booked *before* subcriptions of `type="contract"`
- Subscriptions are booked from `total` (`total -= subscription.calc(last_day_avrg)`) and added to receiver (`receiver.total += subscription.calc(receiver.last_day_avrg`)
- If a subscription of `type=sr` fails to book (not enough credit), a message is sent to the `AI-WO-A controller` (`AI-WO-A-SERVICE/api/credit-overflow?id=<unique_id>&overflow=<overflow`) 

#### Subscriptions

Attributes: 
- `id: str`
- `receiver: int  # the receivers 'unique_id'`
- `value: int  # value in percentage, might be positive or negative`
- `type: str  # might be "sr", "contract"`
- `priority: int`

Methods: 
- `calc(last_day_average: Float) -> Float`

Logic:
- Subscriptions are booked hourly from the user's account to the `receiver`'s account
- The `calc()` function calculates: `last_day_average * value/100` 

### Api
- `GET /api/credits/total?id=<unique_id>`
- `GET /api/credits/hourly?id=<unique_id>`
- `POST /api/credits/book?id=<unique_id>&receiver=<receiver_id>&value=<value>`
- `GET /api/resource/total?id=<unique_id>&resource=<resource>` (where `resource` is optional, returns n-tuple of total for each resource, if not specified)
- `GET /api/resource/hourly?id=<unique_id>&resource=<resource>` (where `resource` is optional, returns n-tuple of hourly for each resource, if not specified)
- `POST /api/resource/book?id=<unique_id>&receiver=<receiver_id>&resource=<resource>&value=<value>` (where `resource` is optional, expects `value` to be an n-tuple if not specified)
- `GET /api/subscriptions?id=<unique_id>` (where `id` is optional, if not specified, return *all* subscriptions)
- `POST /api/subscription/credit/add?id=<unique_id>` (JSON payload `{'id':'<id>', 'receiver': 'receiver_id', 'value': <value>, 'type': '<type>', 'priority': <priority>}`)
- `POST /api/subscription/resource/add?id=<unique_id>&resource=<resource>` (JSON payload `{'id':'<id>', 'receiver': 'receiver_id', 'value': <value>, 'type': '<type>', 'priority': <priority>}`)
- `POST /api/subscription/remove?id=<unique_id>&subscription=<subscription_id>`
- `POST /api/units/add?id=<unique_id>`
- `POST /api/units/set_credit_production?id<trust_id>&value=<value>` (Sets
- `POST /api/units/set_resource_production?id<trust_id>&resource=<resource>&value=<value>` (Sets `last_dayly_avrg`)
