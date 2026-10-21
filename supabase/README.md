# Supabase (to port)

Postgres schema: users, orders, append-only order status events. Port the EVM
ramp's ordered migrations; store Stellar `G...` payout addresses and pin the
payout address at order-create. RLS deny-by-default (service-role only).
