import polars as pl
import polars.selectors as cs
from vlbenchutil import read_gnutime, read_vogls_timings

targets = ["icarus", "vogls-interpret", "vogls-compile"]

# Sanity Check. All simulators output the correct prime.
for target in targets:
    print(f"Checking output of {target}...")
    sanity_primes = pl.read_lines(f"./log-{target}", name="line").filter(
        pl.col.line.str.strip_chars() == "Output = 7aca0fd9bcd6ec7c9f97466616e6a282"
    )
    assert sanity_primes.height > 0

df = pl.concat(
    [
        read_gnutime(targets[0]),
        read_vogls_timings(targets[1]),
        read_vogls_timings(targets[2]),
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
