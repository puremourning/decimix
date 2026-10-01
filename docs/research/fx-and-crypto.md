# FX and crypto conventions: research for `docs/units.md`

Status: research note, September 2026. It supports the glossary and the
limits in `docs/units.md`. Each claim is marked **VERIFIED** (with a
source number from the Sources section) or **UNVERIFIED** (with the reason).
Live exchange data was fetched from public APIs on 2026-09-30; limits and
prices change over time.

## FX

### Quantity: which currency is the amount in?

- **The amount can be in either currency.** FXall's FIX API defines
  `Currency (15)` on an order as "Dealt currency of the order", with the
  values "Either RRR or YYY" (the base or the terms currency), and
  `OrderQty (38)` as the amount of that currency. VERIFIED [1]. Current FIX
  says `Currency (15)` "identifies currency used for price or quantity
  fields, depending on the asset class". VERIFIED [2].
- The other side's amount is carried separately: FIX `CalculatedCcyLastQty
  (1056)` is the "calculated quantity of the other side of the currency
  trade. Can be derived from LastQty and LastPx". VERIFIED [3]. FXall
  carries a `LegContraQty` / `LegAllocContraAmount` ("Contra amount
  calculated using Provider Quoted rate"). VERIFIED [1].
- FXall even supports "mixed dealt" allocations, where some allocations of
  one order are in the base currency and some in the terms currency, and
  flags amounts as firm or "estimated". VERIFIED [1].
- Interbank amounts cluster at round millions of the base currency.
  UNVERIFIED (from a secondary summary of an academic paper on CLS data [4];
  the PDF could not be read).

So "the order quantity is an amount of the pair's base currency" is the
common case, not the rule: an FX quantity is an amount **plus the currency
it is in**.

### Currency minor units (ISO 4217)

From the official list, published 2026-09-17 by SIX (the ISO 4217
maintenance agency). VERIFIED [5]:

| Minor-unit decimals | Codes |
|---|---|
| 0 (17 codes) | BIF, CLP, DJF, GNF, ISK, JPY, KMF, KRW, PYG, RWF, UGX, UYI, VND, VUV, XAF, XOF, XPF |
| 2 | 139 codes (most currencies) |
| 3 (7 codes) | BHD, IQD, JOD, KWD, LYD, OMR, TND |
| 4 (2 codes) | CLF (Chilean Unidad de Fomento), UYW (Uruguayan wage index unit) |
| none ("N.A.") | XAU, XAG, XPT, XPD (metals), XDR, test and bond-market units |

SWIFT FX confirmations (MT 300/305, field 32B/33B, format `3!a15d`) reject an
amount with more decimals than the currency allows (network rule C03), and
the amount field is at most 15 characters including the decimal comma.
VERIFIED [6] (a mirror of the SWIFT User Handbook; the original needs a
login).

### Price: rates, pips and ticks

- A rate is the amount of the terms (second) currency per one unit of the
  base (first) currency: EUR/USD 1.1250 means 1 EUR = 1.1250 USD. VERIFIED
  (search excerpt of CME's FX course [7]; page timed out on direct fetch).
- **Pips and ticks per pair (Currenex ESP)**: 128 pairs have a pip of
  0.0001 and a tick of 5 decimals (a tenth of a pip, "fractional pip
  quoting"); 17 pairs have a pip of 0.01 and a tick of 3 decimals, and they
  are not only JPY pairs: also HUF and THB pairs (USD/HUF, EUR/THB); 3 pairs
  (EUR/CZK, USD/CZK, USD/MAD) have a pip of 0.001 and 4 decimals. Metals
  (XAU/USD) tick at 3 decimals. Currenex's request-for-stream service allows
  prices "up to eight (8) decimal places". VERIFIED [8].
- **LSEG Matching** (formerly Refinitiv/Reuters Matching): market data and
  IOC orders trade in half pips (0.00005) for AUD/USD and EUR/GBP, resting
  orders in tenths of a pip (0.00001); Scandinavian pairs in 10 pips
  (0.0010) and single pips respectively. VERIFIED [9].
- **EBS** moved from pips to tenths of a pip in 2011 (EUR/USD 0.0001 →
  0.00001, USD/JPY 0.01 → 0.001) and to half pips in 2012. UNVERIFIED
  primary (secondary: academic papers [10]; the EBS dealing rules PDF on
  cmegroup.com timed out).
- Tick sizes are per pair and per venue (even per order type), so they are
  instrument data. No venue found uses more than 8 decimals for a rate.

### Forwards, swaps, NDFs, crosses

- **Forward points** are the difference between the forward (outright) and
  spot rates, spoken in pips but sent as a plain price difference: FIX
  `LastForwardPoints (195)`, "May be a negative value. Expressed in decimal
  form. For example, 61.99 points is expressed and sent as 0.006199".
  `LastSwapPoints (1071)` (far leg minus near leg of a swap) uses the same
  form. VERIFIED [11]. Outright = spot + points. FXall negotiates separate
  precisions per request: `SpotDPS`, `PointDPS` (forward points),
  `AllInDPS` (outright), `BaseAmountDPS`, `ContraAmountDPS`. VERIFIED [1].
- **Benchmark precision (WMR, LSEG)**: spot bid/offer "to a maximum of four
  decimal places"; forward and NDF rates "rounded to a maximum of six
  decimal places", with halves rounded up. VERIFIED [12].
- **NDF fixing rates** are published by a named source per currency: FXall
  carries `LegFixingSource`, `LegFixingTime` and a settlement currency per
  NDF leg. VERIFIED [1]. Examples: FBIL's USD/INR reference rate is
  published to 4 decimals. VERIFIED (FBIL documents via search excerpt
  [13]). Brazil's PTAX to 4 decimals (e.g. 5.2204). UNVERIFIED primary
  (news report; BCB's open data portal not read).
- **Cross rates**: WMR calculates GBP and EUR crosses from USD rates, and
  JPY crosses "may have a factor applied". VERIFIED [12]. How far
  intermediate cross calculations are carried before rounding is not
  published. UNVERIFIED.

### Settlement amounts: rounding the contra amount

No market-wide rule was found. What the sources show:

- The contra amount is computed (amount × rate, or amount ÷ rate when the
  terms currency is dealt) and then held to the currency's minor units;
  SWIFT won't accept more decimals [6].
- FXall negotiates the contra-amount precision per request
  (`ContraAmountDPS`) and has `RoundDownCcy`, a "List of currencies that
  will have amounts truncated". VERIFIED [1]. So some currencies are
  truncated rather than rounded to nearest, and the rule is agreed per
  platform.
- CLS settles the two amounts confirmed by both sides; it does not
  recompute them. UNVERIFIED (secondary [4]; no CLS rulebook is public).

## Crypto

### Centralised exchanges: spot

| Venue | Quantity is | Quote-currency orders | Example step and tick (live) |
|---|---|---|---|
| Binance | base asset, decimal | `quoteOrderQty` on MARKET orders: "the amount the user wants to spend (when buying) or receive (when selling)" of the quote asset [14] | BTCUSDT: step 0.00001, tick 0.01. PEPEUSDT: step 1, tick 0.00000001, price 0.00000427 [15] |
| Coinbase | `base_size`, string | `quote_size` (market orders), string [16] | BTC-USD: base 0.00000001, quote 0.01. SHIB-USD: base 1, quote 0.00000001 [17] |
| Kraken | `volume`, "in terms of the base asset", string | `viqc` flag, "supported only for buy market orders" [18] | PEPE/USD: lot 5 decimals, tick 0.000000001 (9 decimals) [19] |
| OKX | `sz` in base currency | (not checked) | BTC-USDT: lot 0.00000001, tick 0.1 [20] |

All VERIFIED from the cited docs and live instrument endpoints.

- **Most decimals seen.** Binance: every spot price and quantity filter is
  written with 8 decimals, and no spot tick is finer than 0.00000001 across
  1,374 trading symbols [15]. Kraken: 9 price decimals; Kraken's own asset
  ledger keeps XBT and ETH to **10** decimals (finer than a satoshi) [19].
  OKX perpetual PEPE tick 0.000000001 [20]. All VERIFIED. Nothing near 19.
- **Coarse relative precision.** BTTCUSDT trades at 0.00000037 with a tick
  of 0.00000001: two significant digits. VERIFIED [15]. The venue, not the
  number format, limits precision here.
- **An i64 underneath?** Binance's largest `maxQty` is 92,233,720,368.00,
  which is exactly (2⁶³ − 1) ÷ 10⁸ rounded down, and SHIBUSDT's is
  46,116,860,414 = 2⁶² ÷ 10⁸. That suggests a 64-bit integer with 8 implied
  decimals inside the matching engine. UNVERIFIED (inference from the
  numbers; Binance doesn't document it).

### Derivatives

- **Contracts, not coins.** OKX `BTC-USDT-SWAP` is linear, one contract =
  `ctVal` 0.01 BTC, settled in USDT; `BTC-USD-SWAP` is inverse, one
  contract = 100 USD, settled in BTC; `PEPE-USDT-SWAP`: one contract =
  10,000,000 PEPE. VERIFIED [20]. CME Bitcoin futures: 5 BTC per contract,
  $5 tick; Micro Bitcoin 0.1 BTC. VERIFIED (cmegroup.com spec pages via
  search excerpt [21]; direct fetch timed out).
- **Deribit** `amount`: "For perpetual and inverse futures the amount is in
  USD units. For options and linear futures it is the underlying base
  currency coin"; `contracts` may be sent instead. VERIFIED [22].
  BTC-PERPETUAL: `instrument_type` "reversed" (inverse), contract size 10
  USD, tick 0.5, settles in BTC. VERIFIED [23].
- **Inverse (BitMEX XBTUSD)**: "each contract is worth 1 USD of Bitcoin"
  [24]; the API gives `isInverse: true`, `multiplier: -100000000`,
  `settlCurrency: "XBt"`, i.e. P&L is computed in whole satoshis. VERIFIED
  [25]. A contract's value in BTC is 1 ÷ price, which never terminates in
  decimal (1 ÷ 65,432.1), so valuing an inverse position always rounds.
- **Quanto (BitMEX ETHUSD)**: `isQuanto: true`, `multiplier: 100`, settles
  in XBt: each 1 USD move in ETH is worth 100 satoshis (0.000001 XBT) per
  contract, whatever the BTC/USD rate. VERIFIED [25], [26].
- **Multiplied tickers.** Binance USD-M futures list `1000SHIBUSDT`,
  `1000PEPEUSDT`, `1000BONKUSDT`, `1MBABYDOGEUSDT`, `1000000MOGUSDT` and
  more, with `baseAsset` set to the synthetic unit (`"1000PEPE"`,
  `"1000000MOG"`); 1000PEPEUSDT ticks at 0.0000001 in whole contracts.
  Binance spot also lists `1000SATSUSDT`. VERIFIED [27], [15]. These exist
  to give sensible prices and ticks, not because the supply is too large:
  PEPE's whole supply (4.2 × 10¹⁴) fits a `Dec19` easily (below).

### On-chain base units

| Chain / asset | Base unit | Integer type | Source |
|---|---|---|---|
| Bitcoin | satoshi = 10⁻⁸ BTC; max 21,000,000 BTC | `int64_t CAmount` | [28] VERIFIED |
| Ethereum | wei = 10⁻¹⁸ ETH (gwei = 10⁻⁹) | 256-bit | [29] VERIFIED |
| ERC-20 | `decimals()` is `uint8` (0–255) and OPTIONAL; amounts, balances and `totalSupply` are `uint256` | [30] VERIFIED |
| Solana | lamport = 10⁻⁹ SOL; SPL mint `decimals: u8`, `supply: u64`, account `amount: u64` | [31], [32] VERIFIED |
| NEAR | yoctoNEAR = 10⁻²⁴ NEAR; `NearToken` wraps a `u128` | [33] VERIFIED |

Real `decimals` and `totalSupply`, read on-chain on 2026-09-30 [34]
(VERIFIED):

| Token | Decimals | Total supply (whole tokens) | Base units |
|---|---|---|---|
| GUSD (Gemini dollar) | 2 | 3.9 × 10⁷ | 3.9 × 10⁹ |
| USDC / USDT | 6 | 4.9 × 10¹⁰ / 8.8 × 10¹⁰ | ~10¹⁷ |
| WBTC | 8 | 1.2 × 10⁵ | 1.2 × 10¹³ |
| SHIB | 18 | 1.0 × 10¹⁵ | 1.0 × 10³³ |
| PEPE | 18 | 4.2 × 10¹⁴ | 4.2 × 10³² |
| Kishu Inu | 9 | 1.0 × 10¹⁷ | 1.0 × 10²⁶ |
| BabyDoge (BSC) | 9 | 4.2 × 10¹⁷ | 4.2 × 10²⁶ |
| YAM-V2 | **24** | 3.7 × 10⁶ | 3.7 × 10³⁰ |

- Tokens with more than 18 decimals exist (YAM-V2, 24) and with as few as 2
  (GUSD). VERIFIED [34], [35].
- **No traded token with more than 1.7 × 10¹⁹ whole units was found.** The
  largest found is BabyDoge at 4.2 × 10¹⁷. `uint256` allows far more, so
  such tokens can exist, but whether any trades on a venue is UNVERIFIED.
- **"Infinite approval"**: OpenZeppelin's ERC-20 treats an allowance of
  `type(uint256).max` (2²⁵⁶ − 1) as unlimited and never decreases it
  [36]. But UNI and COMP store `type(uint96).max` (2⁹⁶ − 1 ≈ 7.9 × 10²⁸)
  when asked for `uint256(-1)` [35]. VERIFIED. That value fits a `u128`, so
  a "≥ 2¹²⁸ means unlimited" rule misses it: at 18 decimals it would read
  as a real 79 billion tokens.

## Implications for decimix / `docs/units.md`

**Glossary corrections.**

- *FX asset unit*: change "the order quantity is an amount of the pair's
  base currency" to "an amount of either currency of the pair (the dealt
  currency), usually the base". The quantity needs its currency alongside,
  as FIX `Currency (15)` does.
- *Minor unit*: holds. Add that 4-decimal (CLF, UYW) and no-decimal
  (metals, XDR) codes exist, and that the pip is not "2nd decimal for JPY
  only": HUF, THB and CZK pairs differ too. Pips and ticks are per-pair
  instrument data.
- *Crypto asset unit*: holds for spot (base asset, decimal string), but
  add: market orders can be sized in the quote currency (`quoteOrderQty`,
  `quote_size`, `viqc`); derivatives count contracts (OKX, CME), USD
  (Deribit inverse, BitMEX), or coins (Deribit linear); and the "asset" may
  be a synthetic multiple ("1000PEPE", "1000000MOG").
- *Limits table*: "some tokens have more than 1.7 × 10¹⁹ whole units" is
  unconfirmed (largest found 4.2 × 10¹⁷), and "1000SHIB"-style contracts
  are about price readability, not range. The "unlimited" sentinel must
  also cover 2⁹⁶ − 1.

**Fits `Dec19` (in asset units, as quoted).** Every CEX price and quantity
seen (at most 10 decimals). Every FX rate (at most 8), forward points in
decimal form (0.006199), and every minor-unit amount. BTC in satoshi
precision (8 dp), ETH in wei (18 dp), SOL (9 dp). Every token supply found
in whole tokens (4.2 × 10¹⁷ < 1.7 × 10¹⁹). Notionals: all 21 million BTC
at 3 × 10⁹ VND each is 6.3 × 10¹⁶, and BabyDoge's whole supply at 10⁻⁹ USD
is 0.42: all far inside range.

**Doesn't fit, or needs care.**

- Base-unit integers, as the note says: SHIB's supply is 10³³ base units
  (fits `u128`/`i128`, not `Dec19`); a balance above ~17 ETH in wei.
- 24-decimal assets (NEAR, YAM-V2): the last 5 digits need base units or a
  named rounding.
- Products lose exactness: an 18-decimal quantity × a 9-decimal price has
  27 decimals, so `mul` with a `Round` is needed, as the API rules require.
  Inverse contracts (1 ÷ price) and dealing in the terms currency (amount ÷
  rate) always round, and venues truncate some currencies, so the rounding
  mode is part of each venue's rules, including toward-zero.
- Raw DEX pool prices (base unit per base unit) can go below 10⁻¹⁹: a
  token worth $10⁻⁸ with 18 decimals, priced in 6-decimal USDC, is
  10⁻⁸ × 10⁶ ÷ 10¹⁸ = 10⁻²⁰ USDC base units per token base unit. Price in
  asset units instead (10⁻⁸), per the note's principle. (Derived from the
  definitions above; no venue checked.)

## Sources

1. FXall FX Trading FIX API v5.1: https://developers.lseg.com/content/dam/devportal/api-families/fx-venues/cash-rfq-fix/refinitiv_fxall_fx_trading_fix_api.pdf
2. FIX Latest, Currency (15): https://orchimate.org/fixtrading/fix-latest/fields/Currency
3. FIX 5.0 SP2, CalculatedCcyLastQty (1056): https://www.onixs.biz/fix-dictionary/5.0.sp2/tagnum_1056.html
4. Hasbrouck & Levich, FX Liquidity and Market Metrics (CLS data): https://pages.stern.nyu.edu/~jhasbrou/Research/Hasbrouck-Levich%20v25.pdf
5. ISO 4217 List One (SIX): https://www.six-group.com/dam/download/financial-information/data-center/iso-currrency/lists/list-one.xml
6. SWIFT MT 305 field 33B (UHB mirror): https://pinas.synology.me/Swift_Manual/2015/books/us3ma/aig018.htm
7. CME, Understanding FX Quote Conventions: https://www.cmegroup.com/education/courses/introduction-to-fx/understanding-fx-quote-conventions
8. Currenex, Supported Currency Pairs and Tenors (July 2024): https://www.currenex.com/content/6-support/documentation/currenex/CX_Supported_CCYPairs_Tenors.pdf
9. LSEG, Price Elision on Matching: https://www.lseg.com/en/fx/announcement/launches-price-elision-matching
10. Tick size reduction and price clustering in a FX order book: https://ar5iv.labs.arxiv.org/html/1307.5440
11. FIX 5.0 SP2 LastForwardPoints (195), LastSwapPoints (1071): https://www.onixs.biz/fix-dictionary/5.0.sp2/tagnum_195.html, https://www.onixs.biz/fix-dictionary/5.0.sp2/tagnum_1071.html
12. WMR FX Benchmarks methodology v30 (Jan 2026): https://www.lseg.com/content/dam/ftse-russell/en_us/documents/ground-rules/wmr-fx-methodology.pdf
13. FBIL Reference Rate: https://www.fbil.org.in/uploads/FBIL_REFERENCE_RATE_FINAL_fa318d5727.pdf
14. Binance spot trading endpoints: https://developers.binance.com/docs/binance-spot-api-docs/rest-api/trading-endpoints
15. Binance spot `exchangeInfo` and `ticker/price` (live): https://api.binance.com/api/v3/exchangeInfo; filters doc: https://developers.binance.com/docs/binance-spot-api-docs/filters
16. Coinbase Advanced Trade, Create Order: https://docs.cdp.coinbase.com/api-reference/advanced-trade-api/rest-api/orders/create-order
17. Coinbase Exchange products (live): https://api.exchange.coinbase.com/products/SHIB-USD
18. Kraken Add Order: https://docs.kraken.com/api/docs/rest-api/add-order/
19. Kraken AssetPairs and Assets (live): https://api.kraken.com/0/public/AssetPairs?pair=XBTUSD,PEPEUSD, https://api.kraken.com/0/public/Assets
20. OKX instruments (live): https://www.okx.com/api/v5/public/instruments?instType=SWAP&instId=PEPE-USDT-SWAP
21. CME Bitcoin / Micro Bitcoin futures specs: https://www.cmegroup.com/markets/cryptocurrencies/bitcoin/bitcoin/specs, https://www.cmegroup.com/markets/cryptocurrencies/bitcoin/micro-bitcoin/specs
22. Deribit private/buy: https://docs.deribit.com/api-reference/trading/private-buy
23. Deribit get_instrument (live): https://www.deribit.com/api/v2/public/get_instrument?instrument_name=BTC-PERPETUAL
24. BitMEX XBTUSD contract page: https://www2.bitmex.com/app/contract/XBTUSD
25. BitMEX instrument API (live): https://www.bitmex.com/api/v1/instrument?symbol=XBTUSD
26. BitMEX blog, Hedging a Quanto Perpetual Swap: https://blog.bitmex.com/hedging-a-quanto-perpetual-swap/
27. Binance USD-M futures `exchangeInfo` (live): https://fapi.binance.com/fapi/v1/exchangeInfo
28. Bitcoin Core `consensus/amount.h`: https://github.com/bitcoin/bitcoin/blob/master/src/consensus/amount.h
29. ethereum.org, Intro to ether: https://ethereum.org/en/developers/docs/intro-to-ether/
30. EIP-20: https://eips.ethereum.org/EIPS/eip-20
31. Solana terminology: https://solana.com/docs/references/terminology
32. SPL Token `interface/src/state.rs`: https://github.com/solana-program/token/blob/main/interface/src/state.rs
33. `near-token` crate source: https://github.com/near/near-token-rs/blob/main/src/lib.rs
34. On-chain `eth_call` of `totalSupply()`/`decimals()` via https://ethereum-rpc.publicnode.com and https://bsc-rpc.publicnode.com (SHIB 0x95aD…C4cE, PEPE 0x6982…1933, USDT 0xdAC1…1ec7, USDC 0xA0b8…eB48, GUSD 0x056F…d5Cd, WBTC 0x2260…C599, YAM-V2 0xAba8…aD8A, Kishu 0xA2b4…817D, BabyDoge BSC 0xc748…e8de)
35. Weird ERC-20 tokens: https://github.com/d-xo/weird-erc20
36. OpenZeppelin ERC20.sol `_spendAllowance`: https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/token/ERC20/ERC20.sol
