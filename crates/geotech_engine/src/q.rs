//! Q-system (Barton et al. 1974; NGI handbook 2015).

/// Validated Q parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QParams {
    pub rqd: f64,
    pub jn: f64,
    pub jr: f64,
    pub ja: f64,
    pub jw: f64,
    pub srf: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QError {
    OutOfRange(&'static str, f64),
}

impl std::fmt::Display for QError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let QError::OutOfRange(p, v) = self;
        write!(f, "{p} = {v} is outside the Q-system tables")
    }
}
impl std::error::Error for QError {}

fn check(name: &'static str, v: f64, lo: f64, hi: f64) -> Result<f64, QError> {
    if v.is_finite() && v >= lo && v <= hi {
        Ok(v)
    } else {
        Err(QError::OutOfRange(name, v))
    }
}

/// RQD below 10 is taken as 10 in Q (NGI 2015).
fn rqd_q(rqd: f64) -> f64 {
    rqd.max(10.0)
}

/// `Q' = (RQD/Jn)(Jr/Ja)` (no water or stress terms).
pub fn q_prime(rqd: f64, jn: f64, jr: f64, ja: f64) -> Result<f64, QError> {
    let rqd = check("RQD", rqd, 0.0, 100.0)?;
    let jn = check("Jn", jn, 0.5, 20.0)?;
    let jr = check("Jr", jr, 0.5, 4.0)?;
    let ja = check("Ja", ja, 0.75, 20.0)?;
    Ok(rqd_q(rqd) / jn * jr / ja)
}

impl QParams {
    pub fn q(&self) -> Result<f64, QError> {
        let qp = q_prime(self.rqd, self.jn, self.jr, self.ja)?;
        let jw = check("Jw", self.jw, 0.05, 1.0)?;
        let srf = check("SRF", self.srf, 0.5, 400.0)?;
        Ok(qp * jw / srf)
    }
}

/// Q class 1 (exceptionally good) .. 9 (exceptionally poor).
pub fn q_class(q: f64) -> u8 {
    match q {
        q if q > 400.0 => 1,
        q if q > 100.0 => 2,
        q if q > 40.0 => 3,
        q if q > 10.0 => 4,
        q if q > 4.0 => 5,
        q if q > 1.0 => 6,
        q if q > 0.1 => 7,
        q if q > 0.01 => 8,
        _ => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_and_q_prime() {
        // RQD 80, Jn 9 (three sets), Jr 1.5, Ja 1, Jw 1, SRF 1 -> 13.33 (good).
        let p = QParams {
            rqd: 80.0,
            jn: 9.0,
            jr: 1.5,
            ja: 1.0,
            jw: 1.0,
            srf: 1.0,
        };
        assert!((p.q().unwrap() - 13.333).abs() < 1e-3);
        assert_eq!(q_class(p.q().unwrap()), 4);
        // Low RQD floors at 10.
        assert!((q_prime(4.0, 1.0, 1.0, 1.0).unwrap() - 10.0).abs() < 1e-12);
        // Water and stress reduce Q.
        let wet = QParams { jw: 0.5, srf: 2.5, ..p };
        assert!((wet.q().unwrap() - 13.333 * 0.5 / 2.5).abs() < 1e-3);
    }

    #[test]
    fn out_of_table_values_are_refused() {
        assert!(q_prime(80.0, 25.0, 1.0, 1.0).is_err());
        assert!(q_prime(80.0, 9.0, 5.0, 1.0).is_err());
        assert!(QParams {
            rqd: 80.0,
            jn: 9.0,
            jr: 1.5,
            ja: 1.0,
            jw: 1.2,
            srf: 1.0
        }
        .q()
        .is_err());
        assert_eq!(q_class(0.005), 9);
        assert_eq!(q_class(500.0), 1);
    }
}
