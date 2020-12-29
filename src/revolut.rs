use crate::prelude::*;
use crate::revolut::api::Transaction;
use crate::{AccountType, ConnectedProvider};
use std::fmt::{Debug, Formatter};

const HEADER_FAIL: &str = "Header should be valid";

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub display_name: String,
    pub device_id: String,
    pub username: String,
    pub password: String,
}

pub fn initialize(token: &mut Token) -> (bool, Result<Box<dyn ConnectedProvider>>) {
    let mut client = Client::new(new_rest_client(&token.device_id));
    client.auth(&token);

    let accounts = client.get_accounts();
    let transactions = client.get_transactions();
    let beneficiaries = client.get_beneficiaries();

    (
        false,
        Ok(Box::new(RevolutProvider {
            accounts,
            beneficiaries,
            transactions: transactions.transactions,
        })),
    )
}

pub mod auth {
    use super::*;

    #[derive(Serialize, Deserialize, StructOpt)]
    #[structopt(rename_all = "kebab-case")]
    pub struct SigninRequest {
        pub phone: String,
        pub password: String,
    }

    #[derive(Serialize, Deserialize, StructOpt)]
    pub struct ConfirmSigninRequest {
        pub phone: String,
        pub code: String,
    }

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ConfirmSigninResponse {
        user: User,
        access_token: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct User {
        id: String,
    }

    impl RestPath<()> for SigninRequest {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("signin".to_string())
        }
    }

    impl RestPath<()> for ConfirmSigninRequest {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("signin/confirm".to_string())
        }
    }
}

pub mod api {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Accounts {
        pub pockets: Vec<Account>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Account {
        pub id: String,
        pub balance: i64,
        pub currency: String,
    }

    impl RestPath<()> for Accounts {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("user/current/wallet".to_string())
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Beneficiary {
        pub id: String,
        pub first_name: String,
        pub last_name: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    #[serde(transparent)]
    pub struct Beneficiaries {
        pub beneficiaries: Vec<Beneficiary>,
    }

    impl RestPath<()> for Beneficiaries {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("user/current/accounts".to_string())
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(transparent)]
    pub struct Transactions {
        pub transactions: Vec<Transaction>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Transaction {
        pub id: String,
        #[serde(with = "chrono::serde::ts_milliseconds", rename = "startedDate")]
        pub date: UtcDateTime,
        pub account: TransactionAccount,
        pub amount: i64,
        pub rate: Option<f64>,
        pub description: Option<String>,
        pub merchant: Option<TransactionMerchant>,
        pub beneficiary: Option<TransactionBeneficiary>,
        pub entry_mode: Option<String>,
        #[serde(rename = "type")]
        pub ty: String,
        pub tag: Option<String>,
        pub direction: Option<String>,
        pub counterpart: Option<TransactionCounterpart>,
        pub state: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct TransactionCounterpart {
        pub account: Option<TransactionAccount>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct TransactionBeneficiary {
        pub id: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct TransactionAccount {
        pub id: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct TransactionMerchant {
        pub name: String,
    }

    impl RestPath<()> for Transactions {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("user/current/transactions".to_string())
        }
    }
}

const HOST: &str = "https://api.revolut.com";

pub fn new_rest_client(device_id: &str) -> RestClient {
    let mut rc = RestClient::builder().build(HOST).unwrap();

    rc.set_header("X-Client-Version", "6.6.2")
        .expect(HEADER_FAIL);
    rc.set_header("X-Api-Version", "1").expect(HEADER_FAIL);
    rc.set_header("X-Device-Id", device_id).expect(HEADER_FAIL);
    rc.set_header("X-Device-Model", "iPhone10,1")
        .expect(HEADER_FAIL);
    rc.set_header(
        "User-Agent",
        "Revolut/com.revolut.revolut (iPhone; iOS 11.1)",
    )
    .expect(HEADER_FAIL);

    rc
}

pub struct Client {
    rc: RestClient,
}

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Client {{ }}")
    }
}

impl Client {
    pub fn new(rc: RestClient) -> Client {
        Client { rc }
    }

    pub fn signin(&mut self, sr: &auth::SigninRequest) {
        let res: Result<(), _> = self.rc.post((), sr);
        println!("{:?}", res);
    }

    pub fn confirm_signin(&mut self, sr: &auth::ConfirmSigninRequest) {
        let res: Result<HashMap<String, String>, _> = self.rc.post_capture((), sr);
        println!("{:?}", res);
    }

    pub fn auth(&mut self, auth: &Token) {
        self.rc.set_auth(&auth.username, &auth.password);
    }

    pub fn get_transactions(&mut self) -> api::Transactions {
        use chrono::{Duration, Utc};
        let date = Utc::now() - Duration::days(30);
        let from = format!("{}", date.timestamp() * 1000);
        let res: Result<_, _> = self.rc.get_with((), &[("from", &from)]);
        res.unwrap()
    }

    pub fn get_accounts(&mut self) -> api::Accounts {
        let res: Result<_, _> = self.rc.get(());
        res.unwrap()
    }

    pub fn get_beneficiaries(&mut self) -> api::Beneficiaries {
        let res: Result<_, _> = self.rc.get(());
        res.unwrap()
    }
}

struct RevolutProvider {
    accounts: api::Accounts,
    beneficiaries: api::Beneficiaries,
    transactions: Vec<Transaction>,
}

impl Debug for RevolutProvider {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "RevolutProvider with {} accounts",
            self.accounts.pockets.len()
        )
    }
}

impl ConnectedProvider for RevolutProvider {
    fn get_accounts(&mut self) -> Result<Vec<crate::Account>> {
        Ok(self
            .accounts
            .pockets
            .iter()
            .map(|acc| crate::Account {
                account_id: acc.id.clone(),
                currency: acc.currency.clone(),
                display_name: acc.currency.clone(),
                balance: acc.balance * 10,
                ty: AccountType::Account,
            })
            .collect())
    }

    fn get_transactions(&mut self, acc: &crate::Account) -> Result<Vec<crate::Transaction>> {
        Ok(self
            .transactions
            .iter()
            .filter(|it| it.account.id == acc.account_id)
            .filter(|it| it.ty != "EXCHANGE" || it.direction.as_deref() == Some("buy"))
            .filter(|it| {
                it.ty != "CARD_PAYMENT"
                    || (match it.state.as_deref() {
                        Some("COMPLETED") => true,
                        Some("PENDING") => true,
                        _ => false,
                    })
            })
            .map(|tran| {
                let transaction_id = tran.id.clone();
                let payee_name =
                    if let Some(merchant) = tran.merchant.as_ref().map(|it| it.name.clone()) {
                        Some(merchant)
                    } else if let Some(beneficiary) = tran.beneficiary.as_ref().map(|it| &it.id) {
                        self.beneficiaries
                            .beneficiaries
                            .iter()
                            .find(|it| &it.id == beneficiary)
                            .map(|it| format!("{} {}", it.first_name, it.last_name))
                    } else if Some("GOOGLE_PAY") == tran.entry_mode.as_deref() {
                        Some("Google Pay Topup".to_string())
                    } else if "TOPUP" == &tran.ty {
                        tran.description
                            .as_ref()
                            .map(|it| it.trim_start_matches("Payment from ").to_string())
                    } else if "FEE" == &tran.ty {
                        match tran.tag.as_deref() {
                            Some("insurance") => Some("Revolut Insurance".to_string()),
                            _ => Some("Revolut".to_string()),
                        }
                    } else if "EXCHANGE" == &tran.ty {
                        //transaction_id = format!("EXCHANGE_{}", tran.date.timestamp());
                        tran.counterpart
                            .as_ref()
                            .and_then(|it| it.account.as_ref())
                            .map(|acc| acc.id.to_string())
                    } else {
                        None
                    };
                crate::Transaction {
                    transaction_id,
                    timestamp: tran.date,
                    amount: tran.amount * 10,
                    description: tran.description.clone().unwrap_or_else(|| "".to_string()),
                    payee_name,
                    category: None,
                }
            })
            .collect())
    }
}
