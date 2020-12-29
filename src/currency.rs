use crate::prelude::*;
use chrono::{NaiveDate, Utc};
use std::io::{Cursor, Read};

pub type Rate = f32;

pub fn load_currency_converter() -> Result<CurrencyConverter> {
    // https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist.zip
    //use bytes::buf::buf::Buf;
    use csv::Reader;
    use reqwest::get;

    let mut rates: HashMap<UtcDate, Vec<Rate>> = HashMap::new();
    let mut resp = get("https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist.zip")?;

    let mut buf = vec![];
    resp.read_to_end(&mut buf)?;

    let mut zf = zip::read::ZipArchive::new(Cursor::new(buf))?;

    let mut reader = Reader::from_reader(zf.by_index(0)?);
    let headers: Vec<&str> = reader.headers()?.iter().collect();
    let currencies = headers[1..].iter().map(|it| it.to_string()).collect();
    for result in reader.records() {
        let result = result?;
        let records: Vec<&str> = result.iter().collect();

        if records.len() < 2 {
            continue;
        }

        let date = NaiveDate::parse_from_str(records[0], "%Y-%m-%d")?;
        let date = UtcDate::from_utc(date, Utc);

        rates.insert(
            date,
            (&records[1..])
                .iter()
                .map(|it| it.parse::<Rate>().unwrap_or(0.0))
                .collect(),
        );
    }

    Ok(CurrencyConverter { currencies, rates })
}

pub struct CurrencyConverter {
    currencies: Vec<String>,
    rates: HashMap<UtcDate, Vec<Rate>>,
}

const EUR: &str = "EUR";

impl CurrencyConverter {
    fn get_index(&self, cur: &str) -> Option<usize> {
        if cur.eq_ignore_ascii_case(EUR) {
            return None;
        }
        self.currencies
            .iter()
            .position(|it| it.eq_ignore_ascii_case(cur))
    }

    pub fn get_rate(&self, mut date: UtcDate, source: &str, dest: &str) -> Option<Rate> {
        let mut offset = 7;
        let rates = loop {
            match self.rates.get(&date) {
                None => {
                    offset -= 1;
                    if offset <= 0 {
                        return None;
                    }
                    date = date.checked_sub_signed(chrono::Duration::days(1)).unwrap();
                }
                Some(t) => break t,
            };
        };

        let source = self.get_index(source);
        let dest = self.get_index(dest);

        let source_rate = match source {
            None => 1.0,
            Some(i) => rates[i],
        };
        let dest_rate = match dest {
            None => 1.0,
            Some(i) => rates[i],
        };

        Some(dest_rate / source_rate)
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use chrono::Utc;

    #[test]
    fn test_converter() -> Result<()> {
        let converter = super::load_currency_converter()?;
        assert_eq!(1.0, converter.get_rate(Utc::today(), "EUR", "EUR").unwrap());
        assert_eq!(1.0, converter.get_rate(Utc::today(), "USD", "USD").unwrap());

        Ok(())
    }
}
