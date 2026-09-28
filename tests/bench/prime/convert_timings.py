import polars as pl
import polars.selectors as cs
from vlbenchutil import read_gnutime, read_vogls_timings

targets = ["verilator", "icarus", "vogls-interpret", "vogls-compile"]

# Sanity Check. All simulators output the correct prime.
for target in targets:
    print(f"Checking output of {target}...")
    sanity_primes = pl.read_lines(f"./log-{target}", name="line").filter(
        pl.col.line.str.starts_with("Prime =")
    )
    assert sanity_primes.height > 0
    assert (
        sanity_primes.select(pl.col.line.str.strip_chars().str.ends_with(" 7919"))
        .to_series()
        .all()
    )

df = pl.concat(
    [
        read_gnutime(targets[0]),
        read_gnutime(targets[1]),
        read_vogls_timings(targets[2]),
        read_vogls_timings(targets[3]),
    ],
    how="horizontal",
)

results = df.select(
    pl.all().filter(pl.row_index() % 2 == 0).mean().name.suffix("_compile_mean"),
    pl.all().filter(pl.row_index() % 2 == 0).std().name.suffix("_compile_std"),
    pl.all().filter(pl.row_index() % 2 == 1).mean().name.suffix("_simulate_mean"),
    pl.all().filter(pl.row_index() % 2 == 1).std().name.suffix("_simulate_std"),
).select(cs.starts_with(target) for target in targets)
results.write_csv("timings.csv")
print(results)
