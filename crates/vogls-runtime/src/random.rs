//! Implementation of the Verilog 2005 17.9 Probabilistic distribution functions

#[derive(Clone, Copy)]
pub enum RandomWarning {
    ChiSquareNonPositiveDegreeOfFreedom,
    ErlangNonPositiveK,
    ExponentialNonPositiveMean,
    PoissonNonPositiveMean,
    TNonPositiveDegOfFreedom,
}

impl RandomWarning {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChiSquareNonPositiveDegreeOfFreedom => {
                "Chi_square distribution must have positive degree of freedom"
            }
            Self::ErlangNonPositiveK => "k-stage erlangian distribution must have positive k",
            Self::ExponentialNonPositiveMean => {
                "Exponential distribution must have a positive mean"
            }
            Self::PoissonNonPositiveMean => "Poisson distribution must have a positive mean",
            Self::TNonPositiveDegOfFreedom => "t distribution must have positive degree of freedom",
        }
    }
}

/// The Verilog standard uses `long`, which nowadays would mean i64. There is seems to mean i32.
type Long = i32;

#[inline(always)]
fn dist_fixup_f64(r: f64) -> Long {
    if r >= 0.0 {
        (r + 0.5) as Long
    } else {
        -((-r + 0.5) as Long)
    }
}

/*
long
rtl_dist_chi_square(seed, df)
long * seed;
long df;
{
  double r;
  long i;
  if (df > 0) {
    r = chi_square(seed, df);
    if (r >= 0) {
      i = (long)(r + 0.5);
    } else {
      r = -r;
      i = (long)(r + 0.5);
      i = -i;
    }
  } else {
    print_error("WARNING: Chi_square distribution must ",
      "have positive degree of freedom\n");
    i = 0;
  }
  return (i);
}
*/
pub fn rtl_dist_chi_square(seed: &mut Long, df: Long, warning: &mut Option<RandomWarning>) -> Long {
    if df <= 0 {
        *warning = Some(RandomWarning::ChiSquareNonPositiveDegreeOfFreedom);
        return 0;
    }

    dist_fixup_f64(chi_square(seed, df))
}

/*
long
rtl_dist_erlang(seed, k, mean)
long * seed;
long k, mean;
{
  double r;
  long i;
  if (k > 0) {
    r = erlangian(seed, k, mean);
    if (r >= 0) {
      i = (long)(r + 0.5);
    } else {
      r = -r;
      i = (long)(r + 0.5);
      i = -i;
    }
  } else {
    print_error("WARNING: k-stage erlangian distribution ",
      "must have positive k\n");
    i = 0;
  }
  return (i);
}
*/
pub fn rtl_dist_erlang(
    seed: &mut Long,
    k: Long,
    mean: Long,
    warning: &mut Option<RandomWarning>,
) -> Long {
    if k <= 0 {
        *warning = Some(RandomWarning::ErlangNonPositiveK);
        return 0;
    }

    dist_fixup_f64(erlangian(seed, k, mean))
}

/*
long
rtl_dist_exponential(seed, mean)
long * seed;
long mean;
{
  double r;
  long i;
  if (mean > 0) {
    r = exponential(seed, mean);
    if (r >= 0) {
      i = (long)(r + 0.5);
    } else {
      r = -r;
      i = (long)(r + 0.5);
      i = -i;
    }
  } else {
    print_error("WARNING: Exponential distribution must ",
      "have a positive mean\n");
    i = 0;
  }
  return (i);
}
*/
pub fn rtl_dist_exponential(
    seed: &mut Long,
    mean: Long,
    warning: &mut Option<RandomWarning>,
) -> Long {
    if mean <= 0 {
        *warning = Some(RandomWarning::ExponentialNonPositiveMean);
        return 0;
    }
    dist_fixup_f64(exponential(seed, mean))
}

/*
long
rtl_dist_normal(seed, mean, sd)
long * seed;
long mean, sd;
{
  double r;
  long i;
  r = normal(seed, mean, sd);
  if (r >= 0) {
    i = (long)(r + 0.5);
  } else {
    r = -r;
    i = (long)(r + 0.5);
    i = -i;
  }
  return (i);
}
*/
pub fn rtl_dist_normal(seed: &mut Long, mean: Long, sd: Long) -> Long {
    dist_fixup_f64(normal(seed, mean, sd))
}

/*
long
rtl_dist_poisson(seed, mean)
long * seed;
long mean;
{
  long i;
  if (mean > 0) {
    i = poisson(seed, mean);
  } else {
    print_error("WARNING: Poisson distribution must have a ",
      "positive mean\n");
    i = 0;
  }
  return (i);
}
*/
pub fn rtl_dist_poisson(seed: &mut Long, mean: Long, warning: &mut Option<RandomWarning>) -> Long {
    if mean <= 0 {
        *warning = Some(RandomWarning::PoissonNonPositiveMean);
        return 0;
    }

    poisson(seed, mean)
}

/*
long
rtl_dist_t(seed, df)
long * seed;
long df;
{
  double r;
  long i;
  if (df > 0) {
    r = t(seed, df);
    if (r >= 0) {
      i = (long)(r + 0.5);
    } else {
      r = -r;
      i = (long)(r + 0.5);
      i = -i;
    }
  } else {
    print_error("WARNING: t distribution must have positive ",
      "degree of freedom\n");
    i = 0;
  }
  return (i);
}
*/
pub fn rtl_dist_t(seed: &mut Long, df: Long, warning: &mut Option<RandomWarning>) -> Long {
    if df <= 0 {
        *warning = Some(RandomWarning::TNonPositiveDegOfFreedom);
        return 0;
    }

    dist_fixup_f64(t(seed, df))
}

/*
long
rtl_dist_uniform(seed, start, end)
long * seed;
long start, end;
{
  double r;
  long i;
  if (start >= end) return (start);
  if (end != LONG_MAX) {
    end++;
    r = uniform(seed, start, end);
    if (r >= 0) {
      i = (long) r;
    } else {
      i = (long)(r - 1);
    }
    if (i < start) i = start;
    if (i >= end) i = end - 1;
  } else if (start != LONG_MIN) {
    start--;
    r = uniform(seed, start, end) + 1.0;
    if (r >= 0) {
      i = (long) r;
    } else {
      i = (long)(r - 1);
    }
    if (i <= start) i = start + 1;
    if (i > end) i = end;
  } else {
    r = (uniform(seed, start, end) +
      2147483648.0) / 4294967295.0);
  r = r * 4294967296.0 - 2147483648.0;
  if (r >= 0) {
    i = (long) r;
  } else {
    i = (long)(r - 1);
  }
}
return (i);
}
*/
pub fn rtl_dist_uniform(seed: &mut Long, mut start: Long, mut end: Long) -> Long {
    if start >= end {
        return start;
    }

    if end != Long::MAX {
        end += 1;
        let r = uniform(seed, start, end);
        let i = if r >= 0.0 {
            r as Long
        } else {
            (r - 1.0) as Long
        };
        i.max(start).min(end - 1)
    } else if start != Long::MIN {
        start -= 1;
        let r = uniform(seed, start, end) + 1.0;
        let i = if r >= 0.0 {
            r as Long
        } else {
            (r - 1.0) as Long
        };
        i.max(start + 1).min(end)
    } else {
        // @NOTE: There is a stray `)` here in the reference implementation. Just ignore it.
        let r = (uniform(seed, start, end) + 2147483648.0) / 4294967295.0;
        let r = r * 4294967296.0 - 2147483648.0;
        if r >= 0.0 {
            r as Long
        } else {
            (r - 1.0) as Long
        }
    }
}

/*
static double
uniform(seed, start, end)
long * seed, start, end;
{
  union u_s {
    float s;
    unsigned stemp;
  }
  u;
  double d = 0.00000011920928955078125;
  double a, b, c;
  if (( * seed) == 0)
    *
    seed = 259341593;
  if (start >= end) {
    a = 0.0;
    b = 2147483647.0;
  } else {
    a = (double) start;
    b = (double) end;
  }
  * seed = 69069 * ( * seed) + 1;
  u.stemp = * seed;
  /*
   * This relies on IEEE floating point format
   */
  u.stemp = (u.stemp >> 9) | 0x3f800000;
  c = (double) u.s;
  c = c + (c * d);
  c = ((b - a) * (c - 1.0)) + a;
  return (c);
}
*/
pub fn uniform(seed: &mut Long, start: Long, end: Long) -> f64 {
    const D: f64 = 0.00000011920928955078125f64;
    let a: f64;
    let b: f64;

    if (*seed) == 0 {
        *seed = 259341593;
    }

    if start >= end {
        a = 0.0;
        b = 2147483647.0;
    } else {
        a = start as f64;
        b = end as f64;
    }

    const CONSTANT: Long = 69069;
    *seed = CONSTANT.wrapping_mul(*seed).wrapping_add(1);
    let stemp = *seed as u32;
    let stemp = (stemp >> 9) | 0x3f800000;
    let c = f64::from(f32::from_bits(stemp));
    let c = c + (c * D);
    ((b - a) * (c - 1.0)) + a
}

/*
static double
normal(seed, mean, deviation)
long * seed, mean, deviation;
{
  double v1, v2, s;
  double log(), sqrt();
  s = 1.0;
  while ((s >= 1.0) || (s == 0.0)) {
    v1 = uniform(seed, -1, 1);
    v2 = uniform(seed, -1, 1);
    s = v1 * v1 + v2 * v2;
  }
  s = v1 * sqrt(-2.0 * log(s) / s);
  v1 = (double) deviation;
  v2 = (double) mean;
  return (s * v1 + v2);
}
*/
pub fn normal(seed: &mut Long, mean: Long, deviation: Long) -> f64 {
    let mut v1: f64 = 0.0;
    let mut v2: f64;
    let mut s = 1.0f64;

    while (s >= 1.0) || (s == 0.0) {
        v1 = uniform(seed, -1, 1);
        v2 = uniform(seed, -1, 1);
        s = v1 * v1 + v2 * v2;
    }

    s = v1 * (-2.0 * s.ln() / s).sqrt();
    v1 = deviation as f64;
    v2 = mean as f64;

    s * v1 + v2
}

/*
static double
exponential(seed, mean)
long * seed, mean;
{
  double log(), n;
  n = uniform(seed, 0, 1);
  if (n != 0) {
    n = -log(n) * mean;
  }
  return (n);
}
*/
pub fn exponential(seed: &mut Long, mean: Long) -> f64 {
    let mut n = uniform(seed, 0, 1);
    if n != 0.0 {
        n = -n.ln() * (mean as f64);
    }
    n
}

/*
static long
poisson(seed, mean)
long * seed, mean;
{
  long n;
  double p, q;
  double exp();
  n = 0;
  q = -(double) mean;
  p = exp(q);
  q = uniform(seed, 0, 1);
  while (p < q) {
    n++;
    q = uniform(seed, 0, 1) * q;
  }
  return (n);
}
*/
pub fn poisson(seed: &mut Long, mean: Long) -> Long {
    let mut n: Long = 0;
    let q = -(mean as f64);
    let p = q.exp();
    let mut q = uniform(seed, 0, 1);
    while p < q {
        n += 1;
        q = uniform(seed, 0, 1) * q;
    }
    n
}

/*
static double
chi_square(seed, deg_of_free)
long * seed, deg_of_free;
{
  double x;
  long k;
  if (deg_of_free % 2) {
    x = normal(seed, 0, 1);
    x = x * x;
  } else {
    x = 0.0;
  }
  for (k = 2; k <= deg_of_free; k = k + 2) {
    x = x + 2 * exponential(seed, 1);
  }
  return (x);
}
*/
pub fn chi_square(seed: &mut Long, deg_of_free: Long) -> f64 {
    let mut x: f64;
    if deg_of_free % 2 != 0 {
        x = normal(seed, 0, 1);
        x = x * x;
    } else {
        x = 0.0;
    }
    for _ in (2..=deg_of_free).step_by(2) {
        x = x + 2.0 * exponential(seed, 1);
    }
    x
}

/*
static double
t(seed, deg_of_free)
long * seed, deg_of_free;
{
  double sqrt(), x;
  double chi2 = chi_square(seed, deg_of_free);
  double div = chi2 / (double) deg_of_free;
  double root = sqrt(div);
  x = normal(seed, 0, 1) / root;
  return (x);
}
*/
pub fn t(seed: &mut Long, deg_of_free: Long) -> f64 {
    let chi2 = chi_square(seed, deg_of_free);
    let div = chi2 / (deg_of_free as f64);
    let root = div.sqrt();
    normal(seed, 0, 1) / root
}

/*
static double
erlangian(seed, k, mean)
long * seed, k, mean;
{
  double x, log(), a, b;
  long i;
  x = 1.0;
  for (i = 1; i <= k; i++) {
    x = x * uniform(seed, 0, 1);
  }
  a = (double) mean;
  b = (double) k;
  x = -a * log(x) / b;
  return (x);
}
*/
pub fn erlangian(seed: &mut Long, k: Long, mean: Long) -> f64 {
    let mut x = 1.0;
    for _ in 1..=k {
        x = x * uniform(seed, 0, 1);
    }
    let a = mean as f64;
    let b = k as f64;
    -a * x.ln() / b
}
