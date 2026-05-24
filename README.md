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

## Testing

### Run tests locally (ephemeral MongoDB)

The project includes `scripts/test-with-ephemeral-mongo.sh`.
It will:
- start a temporary MongoDB container
- run `cargo test`
- remove the container when tests finish (or fail)

Run all tests:

```bash
bash scripts/test-with-ephemeral-mongo.sh
```

Run a specific test:

```bash
bash scripts/test-with-ephemeral-mongo.sh test_get_users_returns_all
```

Notes:
- The script sets `TEST_MONGODB_URI` automatically to the temporary container.
- Test databases are unique per test execution to avoid cross-test pollution.
- The container is removed at the end, so test data does not accumulate.

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
- If an individual's `credit` reaches zero, a notice is sent to the `Blackout controller` (`BLACKOUT-SERVICE/api/credit-overflow?id=<unique_id>`) where unique_id is the user's id

Units 
- Never have incoming subscriptions (no "income" via subscriptions),
- Their `last_day_average` can be set via the API and is not calculated at the end of the day

#### Credits

General: 
- A credit can be "money" or "resource"

Attributes: 
- `balance: Float`
- `last_day_avrg: Float  # average hourly credit of the last day` 
- `subscriptions: List<Subscriptions>`
- `history: List<float>`
- `hourly: Float`

Methods: 
- `hourly(incomming_subsctiptions: List<Float>) -> None`
- `calc_avrg() -> None`
- `evaluate() -> None` 

Logic (unique_id is user's id):
1. hourly calculations (evaluate the credits for every user -- except `Units`):
  - For each User, call the `evaluate()` function of each credit (for `credit` and if they have resources, for every resource as well)
    - The `evaluate()` function evaluates all subscriptions
    - Subscriptions are booked from `total` (`total -= subscription.calc(last_day_avrg)`) and added to the receiver (`receiver.total += subscription.calc(receiver.last_day_avrg`)
    - Subscriptions with higher priority are evaluated first
    - Subscriptions with `type="sr"` are *always* booked *before* subcriptions of `type="contract"`
    - If a subscription of `type=sr` fails to book (not enough credit), a message is sent to the `AI-WO-A controller` (`AI-WO-A-SERVICE/api/credit-overflow?id=<unique_id>&overflow=<overflow`)
    - If an individual's credit hits zero, a message is sent to the Blackout Service (`BLACKOUT-SERVICE/api/credit-overflow?id=<unique_id>`) where unique_id is user's id
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

#### Users
- `GET /api/users/{user_id}`

- `GET /api/users`

- `POST /api/users`
request body is 
```json
{
  "userId": "str",
  "userType": "str" // is always "unit" ?
}
```
response body is the full user object

- `PATCH /api/users/{user_id}`: returns 405 if user not of type unit
request body is
```json
{
  "creditType": "str", // resource_name or "money"
  "lastDayAverage": "int / float"
}
```

#### Credits (money or resource)
- `GET /api/users/{user_id}/credits/{credit_type}`: returns 404 if user or credit type not found
- `GET /api/users/{user_id}/credits`: returns list of balance for each credit type.
 Get the total amount of a user's credits. Returns 404 if user does not exist.
response body is
```json
{
  "credits": [
    {
      "creditType": "str", // resource_name or "money"
      "balance": "float",
      "hourly": "float" // hourly income
    }
  ]
}
```

#### Bookings
- `POST /api/users/{user_id}/bookings`: Book credits (money or resources) from one user (`user_id` in the path) to another (`receiver_id` is the receiving user's `user_id`)
```json
{
  "creditType": "str", // resource_name or "money"
  "receiver": "str", // user_id
  "value": "float" // booked value
}
``` 

#### Subscriptions
- `GET /api/users/{user_id}/subscriptions/{subscription_id}`: returns Subscription object. 404 if user or subscription not found
- `GET /api/users/{user_id}/subscriptions`: returns list of Subscription objects. 404 if user not found, empty list if user has no subscriptions
response body is 

```json
{
  "subscriptions": [
    {
      "subscriptionId": "str",
      "receiver": "str", // user_id
      "value": "int", // value in percentage, might be positive or negative`
      "subscriptionType": "str", // "sr" or "contract"
      "priority": "int",
      "creditType": "str" // resource_name or "money"
    }
  ]
}

```

- `POST /api/users/{user_id}/subscriptions`: adds Subscriptions object for user
request body is
```json
{
  "receiver": "str", // user_id
  "value": "int", // value in percentage, might be positive or negative`
  "subscriptionType": "str", // "sr" or "contract"
  "priority": "int",
  "creditType": "str" // resource_name or "money"
}
```
returns full subscription object including id

- `DELETE /api/users/{user_id}/subscriptions/{subscription_id}`: returns 404 if user not found, returns success if subscription not found or successfully deleted

