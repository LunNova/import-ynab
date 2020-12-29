use crate::prelude::*;
use api::*;
use chrono::Utc;
use std::convert::TryInto;
use std::ops::{Add, Deref};
type DateTime = chrono::DateTime<chrono::Utc>;
use crate::config::YnabConfig;
use crate::AccountType;

use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::url::Url;
use oauth2::{
    AccessToken, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};

struct TruelayerProvider {
    display_name: String,
    rest_client: RestClient,
}

impl TruelayerProvider {
    fn test(&mut self) -> Result<()> {
        let metadata: AccessTokenMetadataResponse = self.rest_client.get(())?;
        let metadata = &metadata.results[0];

        let identity: IdentityResponse = self.rest_client.get(())?;
        let identity = &identity.results[0];

        self.display_name = format!(
            "{} {} {} {} {}",
            metadata.provider.display_name,
            identity.full_name,
            metadata.consent_status.as_deref().unwrap_or("no status"),
            metadata
                .consent_expires_at
                .as_deref()
                .unwrap_or("no expiry"),
            metadata.client_id
        );
        Ok(())
    }
}

impl std::fmt::Debug for TruelayerProvider {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(fmt, "TruelayerProvider: {}", &self.display_name)
    }
}

impl crate::ConnectedProvider for TruelayerProvider {
    fn get_accounts(&mut self) -> Result<Vec<crate::Account>> {
        let accounts: Result<AccountsResponse, _> = self.rest_client.get(());
        let cards: Result<CardsResponse, _> = self.rest_client.get(());

        let (accounts, cards) = match (accounts, cards) {
            // TODO second err also?
            (Err(a), Err(_b)) => return Err(a.into()),
            any => any,
        };

        let mut converted_accounts = vec![];

        if let Ok(accounts) = accounts {
            converted_accounts.extend(
                accounts
                    .results
                    .into_iter()
                    .map(|truelayer_acc| {
                        let balance: AccountBalanceResponse =
                            self.rest_client.get(truelayer_acc.account_id.deref())?;
                        Ok(crate::Account {
                            account_id: truelayer_acc.account_id,
                            currency: truelayer_acc.currency,
                            display_name: truelayer_acc.display_name,
                            ty: AccountType::Account,
                            balance: (balance.results[0].current * 1000.0) as i64,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            )
        }

        if let Ok(accounts) = cards {
            converted_accounts.extend(
                accounts
                    .results
                    .into_iter()
                    .map(|truelayer_acc| {
                        let balance: CardBalanceResponse =
                            self.rest_client.get(truelayer_acc.account_id.deref())?;
                        Ok(crate::Account {
                            account_id: truelayer_acc.account_id,
                            currency: truelayer_acc.currency,
                            display_name: truelayer_acc.display_name,
                            ty: AccountType::Card,
                            balance: -(balance.results[0].current * 1000.0) as i64,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            )
        }

        Ok(converted_accounts)
    }

    fn get_transactions(&mut self, acc: &crate::Account) -> Result<Vec<crate::Transaction>> {
        let transactions = match acc.ty {
            AccountType::Account => {
                let transactions: TransactionsResponse =
                    self.rest_client.get(acc.account_id.deref())?;
                transactions.results
            }
            AccountType::Card => {
                let mut transactions: CardTransactionsResponse =
                    self.rest_client.get(acc.account_id.deref())?;
                for transaction in &mut transactions.results {
                    transaction.amount = -transaction.amount;
                }
                transactions.results
            }
        };

        //println!("{:#?}", transactions.iter().take(10).collect::<Vec<_>>());

        Ok(transactions
            .into_iter()
            .map(|truelayer_tran| crate::Transaction {
                transaction_id: truelayer_tran.transaction_id,
                timestamp: truelayer_tran.timestamp,
                amount: (truelayer_tran.amount * 1000f64) as i64,
                payee_name: match truelayer_tran.merchant_name {
                    Some(s) => Some(s),
                    None => truelayer_tran
                        .description
                        .split_whitespace()
                        .next()
                        .map(|it| it.to_string()),
                },
                description: truelayer_tran.description,
                category: None,
            })
            .collect())
    }
}

pub fn initialize(
    ynab_config: &YnabConfig,
    token: &mut Token,
) -> (bool, Result<Box<dyn crate::ConnectedProvider>>) {
    let (refreshed, access_token) = match refresh(ynab_config, token) {
        Ok((refreshed, token)) => (refreshed, token),
        Err(e) => return (false, Err(e)),
    };
    let mut provider = TruelayerProvider {
        display_name: token.display_name.to_string(),
        rest_client: new_rest_client(&access_token),
    };
    if let Err(e) = provider.test() {
        return (refreshed, Err(e));
    }
    token.display_name = provider.display_name.clone();
    (refreshed, Ok(Box::new(provider)))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub display_name: String,
    pub access_token: oauth2::AccessToken,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub access_token_expiry: DateTime,
    pub refresh_token: oauth2::RefreshToken,
}

pub mod api {
    use crate::prelude::*;

    // https://docs.truelayer.com/#retrieve-access_token-metadata
    // https://api.truelayer.com/data/v1/me
    #[derive(Serialize, Deserialize)]
    pub struct AccessTokenMetadata {
        pub client_id: String,
        pub consent_status: Option<String>,
        pub consent_created_at: Option<String>,
        pub consent_expires_at: Option<String>,
        pub provider: Provider,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Identity {
        pub full_name: String,
        #[serde(default)]
        pub emails: Vec<String>,
        #[serde(default)]
        pub phones: Vec<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Account {
        pub account_id: String,
        pub account_type: String,
        pub display_name: String,
        pub currency: String,
        pub account_number: AccountNumber,
        pub provider: Provider,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Card {
        pub account_id: String,
        pub display_name: String,
        pub currency: String,
        pub partial_card_number: String,
        pub name_on_card: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct AccountNumber {
        pub iban: Option<String>,
        pub number: Option<String>,
        pub sort_code: Option<String>,
        pub swift_bic: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Provider {
        pub display_name: String,
        pub logo_uri: String,
        pub provider_id: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Transaction {
        // https://docs.truelayer.com/#retrieve-account-transactions
        // TODO transaction ids are shit
        pub transaction_id: String,
        #[serde(deserialize_with = "crate::serialisation::deserialize")]
        pub timestamp: UtcDateTime,
        pub amount: f64,
        pub description: String,
        pub merchant_name: Option<String>,
        pub meta: Option<HashMap<String, serde_json::value::Value>>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct AccountBalance {
        pub current: f64,
    }

    #[derive(Serialize, Deserialize)]
    pub struct AccessTokenMetadataResponse {
        pub results: Vec<AccessTokenMetadata>,
    }

    impl RestPath<()> for AccessTokenMetadataResponse {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("data/v1/me".to_string())
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct IdentityResponse {
        pub results: Vec<Identity>,
    }

    impl RestPath<()> for IdentityResponse {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("data/v1/info".to_string())
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct AccountsResponse {
        pub results: Vec<Account>,
    }

    impl RestPath<()> for AccountsResponse {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("data/v1/accounts".to_string())
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct CardsResponse {
        pub results: Vec<Card>,
    }

    impl RestPath<()> for CardsResponse {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("data/v1/cards".to_string())
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct TransactionsResponse {
        pub results: Vec<Transaction>,
    }

    impl RestPath<&str> for TransactionsResponse {
        fn get_path(account_id: &str) -> Result<String, restson::Error> {
            Ok(format!("data/v1/accounts/{}/transactions", account_id))
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct CardTransactionsResponse {
        pub results: Vec<Transaction>,
    }

    impl RestPath<&str> for CardTransactionsResponse {
        fn get_path(account_id: &str) -> Result<String, restson::Error> {
            Ok(format!("data/v1/cards/{}/transactions", account_id))
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct AccountBalanceResponse {
        pub results: Vec<AccountBalance>,
    }

    impl RestPath<&str> for AccountBalanceResponse {
        fn get_path(account_id: &str) -> Result<String, restson::Error> {
            Ok(format!("data/v1/accounts/{}/balance", account_id))
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct CardBalanceResponse {
        pub results: Vec<AccountBalance>,
    }

    impl RestPath<&str> for CardBalanceResponse {
        fn get_path(account_id: &str) -> Result<String, restson::Error> {
            Ok(format!("data/v1/cards/{}/balance", account_id))
        }
    }
}

pub fn new_oauth2_client(client_id: &str, client_secret: &str) -> Result<BasicClient> {
    Ok(BasicClient::new(
        ClientId::new(client_id.to_string()),
        Some(ClientSecret::new(client_secret.to_string())),
        AuthUrl::new("https://auth.truelayer.com/".to_string())?,
        Some(TokenUrl::new(
            "https://auth.truelayer.com/connect/token".to_string(),
        )?),
    )
    .set_redirect_url(RedirectUrl::new(
        "https://console.truelayer.com/redirect-page".to_string(),
    )?))
}

pub fn new_rest_client(access_token: &AccessToken) -> RestClient {
    const HOST: &str = "https://api.truelayer.com/";

    let mut rc = RestClient::builder().build(HOST).unwrap();

    let header_fail = "Header should be valid";

    rc.set_header(
        "Authorization",
        &("Bearer ".to_string() + access_token.secret()),
    )
    .expect(header_fail);
    rc.set_header("User-Agent", "moonstruck.dev/import-ynab")
        .expect(header_fail);

    rc
}

pub fn get_auth_url(config: &YnabConfig) -> Result<Url> {
    let client = new_oauth2_client(&config.truelayer_client_id, &config.truelayer_client_secret)?;

    let (url, _token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("accounts".to_string()))
        .add_scope(Scope::new("balance".to_string()))
        .add_scope(Scope::new("info".to_string()))
        .add_scope(Scope::new("offline_access".to_string()))
        .add_scope(Scope::new("transactions".to_string()))
        .add_scope(Scope::new("cards".to_string()))
        .add_scope(Scope::new("direct_debits".to_string()))
        .add_scope(Scope::new("standing_orders".to_string()))
        .add_extra_param("enable_mock", "true")
        .add_extra_param("enable_oauth_providers", "true")
        .add_extra_param("enable_open_banking_providers", "true")
        .add_extra_param("enable_credentials_sharing_providers", "true")
        .url();

    Ok(url)
}

pub fn authorize(
    config: &YnabConfig,
    token: &str,
) -> Result<impl oauth2::TokenResponse<oauth2::basic::BasicTokenType>> {
    let client = new_oauth2_client(&config.truelayer_client_id, &config.truelayer_client_secret)?;
    let token = client
        .exchange_code(AuthorizationCode::new(token.to_string()))
        .request(http_client)
        .map_err(|e| anyhow!("{:?}", e))
        .context("Failed to authorize with truelayer")?;

    Ok(token)
}

pub fn refresh(ynab_config: &YnabConfig, token: &mut Token) -> Result<(bool, AccessToken)> {
    let now = Utc::now();

    if now < token.access_token_expiry {
        return Ok((false, token.access_token.clone()));
    }

    let client = new_oauth2_client(
        &ynab_config.truelayer_client_id,
        &ynab_config.truelayer_client_secret,
    )?;
    let new_token = client
        .exchange_refresh_token(&token.refresh_token)
        .request(http_client)
        .map_err(|e| anyhow!("{:?}", e))
        .context("Error refreshing truelayer token")?;

    token.access_token = new_token.access_token().clone();
    token.refresh_token = new_token.refresh_token().unwrap().clone();
    token.access_token_expiry = calculate_expiry_time(new_token.expires_in().unwrap());

    Ok((true, new_token.access_token().clone()))
}

pub fn calculate_expiry_time(expires_in: std::time::Duration) -> DateTime {
    Utc::now().add(chrono::Duration::seconds(
        expires_in.as_secs().try_into().unwrap(),
    ))
}
