# credit-exchanger
Exchange service for blocs, zones and individuals

The credit-exchanger keeps track of its users' money and resources and has two main tasks: 
1. allow users to "book" credits (money or resources) from one account into the other
2. Evaluate users' subscriptions (hourly):
   a) book subscriptions from the users' account to the `receivers` account
   b) calculate the user's hourly income: the difference between all incoming transactions (other users' subscriptions) and the outgoing transactions (sum of all of the user's subscriptions)

NOTE: Why do we need the "hourly income"? A subscription is not a fixed amount of money (user A pays 500 credits to user B), but rather user A pays 10% of his *last day's average hourly income*!  
This is the third (implicit) task of the credit-exchange: 
3. When the hourly income is calculated (once per hour), store that in the Credits `history`. At the end of the day (`00:00`) calculate the `last_days_average` from the history and empty the history afterwards.

NOTE (on naming): Currently, `credit` refers to the object holding either "money"-credit or "resource"-credit. Every User has `credit` which refers to "money"; some users also have a set of credits for multiple resources. Since this is confusing, we sometimes write "money" to refer to `User.credit`

## Installation 
...
...

## Specification 

### Types

#### Users
There are three types of users: 
- Bloc
- Zone
- Individual
- Units (*Either Trust or Base, but the credit exchanger does not differentiate*) 

Each user has: 
- `unique_id: str`
- `credit: Credit` (money)

Blocs and Zones have: 
- `resources: Dict<resource: str, credit: Credit>`

Individuals
- Do not have `resources`
- If an individual's `credit` reaches zero, a notice is sent to the `Blackout controller` (`BLACKOUT-SERVICE/api/credit-overflow?id=<unique_id>`)

Units 
- Never have incoming subscriptions (no "income" via subscriptions),
- Their `last_day_average` can be set via the API and is not calculated at the end of the day

#### Credits

General: 
- A credit can be "money" or "resource"

Attributes: 
- `total: Float`
- `last_day_avrg: Float  # average hourly credit of the last day` 
- `subscriptions: List<Subscriptions>`
- `history: List<float>`
- `hourly: Float`

Methods: 
- `hourly(incomming_subsctiptions: List<Float>) -> None`
- `calc_avrg() -> None`
- `evaluate() -> None` 

Logic: 
1. hourly calculations (evaluate the credits for every user -- except `Units`):
  - For each User, call the `evaluate()` function of each credit (for `credit` and if they have resources, for every resource as well)
    - The `evaluate()` function evaluates all subscriptions
    - Subscriptions are booked from `total` (`total -= subscription.calc(last_day_avrg)`) and added to the receiver (`receiver.total += subscription.calc(receiver.last_day_avrg`)
    - Subscriptions with higher priority are evaluated first
    - Subscriptions with `type="sr"` are *always* booked *before* subcriptions of `type="contract"`
    - If a subscription of `type=sr` fails to book (not enough credit), a message is sent to the `AI-WO-A controller` (`AI-WO-A-SERVICE/api/credit-overflow?id=<unique_id>&overflow=<overflow`)
    - If an individual's credit hits zero, a message is sent to the Blackout Service (`BLACKOUT-SERVICE/api/credit-overflow?id=<unique_id>`)
  - For each user's credit (except for `Unit`s!!), calculate hourly income (`Unit.hourly()`) (again: for `credit` and if they have resources, for every resource as well)
    - The `hourly(incoming: List<Float>)` function calculates: `sum(incoming) - sum([s.calc(last_day_average) for s in subscriptions]` and the result is stored to `history`.
2. `last_day_avrg` is calculated at `00:00`: `last_day_avrg = avrg(history)` (not for Units!!)

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
- `GET /api/credits/total?id=<unique_id>`: Get the total amount of a `User`'s credits (money, not resources)
- `GET /api/credits/hourly?id=<unique_id>`: Get the hourly income of a `User's credits (money, not resources)
- `POST /api/credits/book?id=<unique_id>&receiver=<receiver_id>&value=<value>`: Book credits (money, not resources) from one user to another (`receiver_id` is the receiving user's `unique_id`)
- `GET /api/resource/total?id=<unique_id>&resource=<resource>` (where `resource` is optional, returns n-tuple of total for each resource, if not specified): Get the total amount of a user's resources (if the user has resources; `Individual`s do *not* have resources). If `resource` is specified: of that specific resource, otherwise an n-tuple with the total of each resource.
- `GET /api/resource/hourly?id=<unique_id>&resource=<resource>` (where `resource` is optional, returns n-tuple of hourly for each resource, if not specified): Get the hourly income of a user's resources (if the user has resources; `Individual`s do *not* have resources). If `resource` is specified: of that specific resource, otherwise an n-tuple with the hourly income of each resource.
- `POST /api/resource/book?id=<unique_id>&receiver=<receiver_id>&resource=<resource>&value=<value>` (where `resource` is optional, expects `value` to be an n-tuple if not specified): Book the resource(s) from one user to another.
- `GET /api/subscriptions?id=<unique_id>&id=<subscription_id>` (where `subscription_id` is optional, if not specified, return *all* subscriptions): Get all subscriptions or one specific subscription (if `subscription_id` is specified`) of a given User. 
- `POST /api/subscription/credit/add?id=<unique_id>` (JSON payload `{'id':'<id>', 'receiver': 'receiver_id', 'value': <value>, 'type': '<type>', 'priority': <priority>}`): Adds a "money"-credit subscription to a given user.
- `POST /api/subscription/resource/add?id=<unique_id>&resource=<resource>` (JSON payload `{'id':'<id>', 'receiver': 'receiver_id', 'value': <value>, 'type': '<type>', 'priority': <priority>}`): Adds a "resource"-credit subscription to a given user.
- `POST /api/subscription/remove?id=<unique_id>&subscription=<subscription_id>`: Removes a subscription from a given user
- `POST /api/units/add?id=<unique_id>`: Adds a new `Unit`
- `POST /api/units/set_credit_production?id=<unique_id>&value=<value>`: Set `last_day_average` "money"-credit for a given user 
- `POST /api/units/set_resource_production?id<unique_id>&resource=<resource>&value=<value>`:  Set `last_day_average` "money"-credit for a given user
