---
crate: minor
---
- Every Modern venue with a spend per customer keeps a patronage tally: the
  nightclub, spa, sky bar and daycare now count their customers like the shop
  and the food hall, and the catalog publishes their `spendPerCustomer`, so a
  frontend's patronage block shows them.
- While walk-ins are off, the amusements, boutique bay, nightclub, spa, sky bar
  and daycare count the customers they served the day before toward the
  population, held to each venue's population (`census_counts_patronage`).
  They add nothing to the population recycling plans for. The conformance
  lock, the frame-day golden and the Modern smoke corpus golden move with it.
- `unitDetail`'s `customersIn` is a number only for a venue people come to in
  person (`Kind::counts_visitors`), and `null` for every other kind, where it
  used to read 0 for venues nobody is ever sent to.
