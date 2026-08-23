# 06 Feedback Rules & Invariant Guardrails — SamarDB

## Non-Negotiable Engineering Rules
1. **Priority Order**: Data safety > Result correctness > Recovery correctness > Tail latency > Throughput > Feature count > Developer convenience.
2. **Never Invert**: A default configuration must never risk losing an acknowledged commit upon crash.
3. **Spec the Bytes Before Writing Them**: All on-disk and on-wire formats get a byte-level table in `docs/format/` with version fields before code.
4. **Deterministic by Construction**: Modules do not access the OS clock, RNG, network, or disk directly; all operations use injectable interfaces.
5. **Idempotence**: Recovery REDO passes must be 100% idempotent.
