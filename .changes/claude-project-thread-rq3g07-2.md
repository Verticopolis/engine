---
crate: patch
---
- In a Classic tower, shops and restaurants take customers in person from
  outside the tower again (`WALK_INS_ON`, `GameMode::shoppers_walk_in`), as
  many a day as the 1994 game served
  (`crowd::walk_ins::daily_walk_ins`: about 25 to 30 for a shop and 35 to 50
  for a restaurant or fast food, half for a venue far from the street). A
  shopper whose way in already has a full car's load waiting stays away,
  and one who waits 15 minutes at the door walks back out; their waits do
  not count toward the tenants' mood. A venue's income and Today's
  patronage still follow the demand. A shop's head count is its shoppers
  inside, as a meal venue's is. A Modern tower takes no walk-ins yet and
  plays as before. Classic conformance checkpoints move; Modern's do not.
- Two speedups that change no result: the crowd remembers each passenger
  route between two segments until the tower changes (`crowd::route_memo`),
  and the hourly pass fills the parking reach once instead of four times
  (`Tower::hold_parking`).
