# Bot (to port)

Telegram chat bot (Grammy). Creates orders and Sentoo payment links; **holds no
signing key**. Port from the EVM ramp, replacing the chain layer with the Stellar
SDK: validate Stellar `G...` addresses, surface the USDC **trustline** requirement
before a buy, and keep the `?ref=` Telegram return link.

Not yet implemented — see [`../docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md).
