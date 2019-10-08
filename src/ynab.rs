use crate::config::Config;
use crate::prelude::*;
use crate::Transaction;
use api::*;
use chrono::Utc;

pub fn sync(config: &mut Config) -> Result<(), Error> {
    let currency_converter = crate::currency::load_currency_converter()?;
    let mut rc = new_rest_client(&config.ynab_config.access_token);
    let ynab_accounts = get_accounts(&mut rc, &config.ynab_config.budget_id)?;
    let budget = get_budget(&mut rc, &config.ynab_config.budget_id)?;
    let currency = &budget.currency_format.iso_code;
    println!("{:#?}", ynab_accounts);

    let mut ided_accounts: HashMap<String, &Account> = HashMap::new();
    for acc in &ynab_accounts {
        let pre = "ACCOUNT_ID=\"";
        if let Some(note) = &acc.note {
            if let Some(idx) = note.find(pre) {
                let rest = &note[(idx + pre.len())..];
                if let Some(end_idx) = rest.find('"') {
                    let id = &rest[0..end_idx];
                    println!("Found account ID {} for {}", id, acc.name);
                    ided_accounts.insert(id.to_string(), acc);
                }
            }
        }
    }

    let mut providers = crate::load_connections(config)?;
    let mut visited_accounts = vec![];
    for provider in &mut providers {
        for account in provider.get_accounts()? {
            let ynab_account = ided_accounts.get(&account.account_id);
            println!(
                "Looking for ynab account with ACCOUNT_ID=\"{}\" for {:?} {}",
                account.account_id, provider, account.display_name
            );
            if let Some(ynab_account) = ynab_account {
                let mut trans = provider.get_transactions(&account)?;
                println!(
                    "Found ynab account {} = {}. {} Transactions to import.",
                    ynab_account.name,
                    ynab_account.balance,
                    trans.len()
                );
                if !account.currency.eq_ignore_ascii_case(currency) {
                    for tran in &mut trans {
                        let rate = currency_converter
                            .get_rate(tran.timestamp.date(), &account.currency, currency)
                            .ok_or_else(|| {
                                failure::err_msg(format!("Missing rates for {:#?}", tran))
                            })?;
                        tran.amount = (tran.amount as crate::currency::Rate * rate) as i64;
                    }
                }
                for tran in &mut trans {
                    if let Some(payee_id) = &tran.payee_name {
                        if let Some(acc) = ided_accounts.get(payee_id) {
                            tran.payee_name = Some(acc.id.clone());
                        }
                    }
                }
                import_transactions(
                    &mut rc,
                    &config.ynab_config.budget_id,
                    &ynab_account.id,
                    trans,
                )?;
                visited_accounts.push(account);
            }
        }
    }

    for account in visited_accounts {
        let ynab_account = ided_accounts.get(&account.account_id);
        if let Some(ynab_account) = ynab_account {
            let ynab_account =
                get_account(&mut rc, &config.ynab_config.budget_id, &ynab_account.id)?;

            let rate = currency_converter
                .get_rate(Utc::today(), &account.currency, currency)
                .ok_or_else(|| failure::err_msg(format!("Missing rates for {:#?}", &account)))?;
            let calc_balance = (account.balance as crate::currency::Rate * rate) as i64;
            println!(
                "Account {} = {}. Expected balance {}",
                ynab_account.name, ynab_account.balance, calc_balance
            );
            if should_reconcile(ynab_account.balance, calc_balance) {
                let correction = crate::Transaction {
                    transaction_id: "correction_".to_string() + &Utc::now().timestamp().to_string(),
                    timestamp: Utc::now(),
                    amount: calc_balance - ynab_account.balance,
                    description: format!(
                        "Reconciliation. {} @ {} = {}",
                        (account.balance as f64 / 1000.0),
                        rate,
                        (calc_balance as f64 / 1000.0)
                    ),
                    category: Some("Inflow: To be Budgeted".to_string()),
                    payee_name: Some("Sync Reconciliation".to_string()),
                };
                import_transactions(
                    &mut rc,
                    &config.ynab_config.budget_id,
                    &ynab_account.id,
                    vec![correction],
                )?;
            }
        }
    }

    Ok(())
}

pub fn new_rest_client(access_token: &str) -> RestClient {
    const HOST: &str = "https://api.youneedabudget.com/";

    let mut rc = RestClient::builder().build(HOST).unwrap();

    let header_fail = "Header should be valid";

    let bearer = &("Bearer ".to_string() + access_token);

    rc.set_header("Authorization", bearer).expect(header_fail);
    rc.set_header("accept", "application/json")
        .expect(header_fail);
    rc.set_header("User-Agent", "moonstruck.dev/import-ynab")
        .expect(header_fail);

    rc
}

pub fn get_accounts(rc: &mut RestClient, budget_id: &str) -> Result<Vec<Account>, Error> {
    let accounts: Wrapper<AccountsResponse> = rc.get(budget_id)?;
    Ok(accounts.data.accounts)
}

pub fn get_account(
    rc: &mut RestClient,
    budget_id: &str,
    account_id: &str,
) -> Result<Account, Error> {
    let accounts: Wrapper<AccountResponse> = rc.get((budget_id, account_id))?;
    Ok(accounts.data.account)
}

pub fn get_budget(rc: &mut RestClient, budget_id: &str) -> Result<Budget, Error> {
    let accounts: Wrapper<BudgetResponse> = rc.get(budget_id)?;
    Ok(accounts.data.budget)
}

pub fn get_payees(rc: &mut RestClient, budget_id: &str) -> Result<Vec<Payee>, Error> {
    let accounts: Wrapper<PayeesResponse> = rc.get(budget_id)?;
    Ok(accounts.data.payees)
}

pub fn import_transactions(
    rc: &mut RestClient,
    budget_id: &str,
    account_id: &str,
    transactions: Vec<Transaction>,
) -> Result<(), Error> {
    let payees = get_payees(rc, budget_id)?;
    for trans in transactions.chunks(50) {
        let trans = trans
            .iter()
            .map(|tran| {
                let mut payee_name = tran.payee_name.clone();
                let mut payee_id: Option<String> = None;
                if let Some(payee) = &payee_name {
                    if let Some(payee) = payees
                        .iter()
                        .find(|it| it.transfer_account_id.as_ref() == Some(payee))
                    {
                        payee_name = None;
                        payee_id = Some(payee.id.clone());
                    }
                }
                NewTransaction {
                    account_id: account_id.to_string(),
                    date: tran.timestamp.date().format("%Y-%m-%d").to_string(),
                    amount: tran.amount,
                    payee_name,
                    payee_id,
                    memo: tran.description.clone(),
                    cleared: "cleared".to_string(),
                    import_id: Some(tran.transaction_id.clone()),
                    category_name: tran.category.clone(),
                }
            })
            .collect();
        rc.post(
            budget_id,
            &NewTransactions {
                transactions: trans,
            },
        )?
    }

    Ok(())
}

fn should_reconcile(acc_balance: i64, calc_balance: i64) -> bool {
    if acc_balance == calc_balance {
        return false;
    }
    if acc_balance == 0 || calc_balance == 0 {
        return true;
    }
    if (acc_balance - calc_balance).abs() > 1000 {
        return true;
    }
    let ratio = acc_balance as f32 / calc_balance as f32;
    let ratio = if ratio < 1.0 { 1.0 / ratio } else { ratio };
    if ratio > 1.01 {
        return true;
    }
    false
}

pub mod api {
    use crate::prelude::*;

    #[derive(Serialize, Deserialize)]
    pub struct Budget {
        pub id: String,
        pub name: String,
        pub currency_format: BudgetCurrency,
    }

    #[derive(Serialize, Deserialize)]
    pub struct BudgetCurrency {
        pub iso_code: String,
    }

    #[derive(Deserialize)]
    pub struct BudgetsResponse {
        pub budgets: Vec<Budget>,
    }

    #[derive(Deserialize)]
    pub struct BudgetResponse {
        pub budget: Budget,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Account {
        pub id: String,
        pub name: String,
        pub note: Option<String>,
        pub balance: i64,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Payee {
        pub id: String,
        pub name: String,
        pub transfer_account_id: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PayeesResponse {
        pub payees: Vec<Payee>,
    }

    #[derive(Deserialize)]
    pub struct AccountsResponse {
        pub accounts: Vec<Account>,
    }

    #[derive(Deserialize)]
    pub struct AccountResponse {
        pub account: Account,
    }

    #[derive(Serialize)]
    pub struct NewTransaction {
        pub account_id: String,
        pub date: String,
        pub amount: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub payee_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub payee_id: Option<String>,
        pub memo: String,
        pub cleared: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub import_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub category_name: Option<String>,
    }

    #[derive(Serialize)]
    pub struct NewTransactions {
        pub transactions: Vec<NewTransaction>,
    }

    impl RestPath<()> for BudgetsResponse {
        fn get_path(_: ()) -> Result<String, restson::Error> {
            Ok("v1/budgets".to_string())
        }
    }

    impl RestPath<&str> for Wrapper<BudgetResponse> {
        fn get_path(budget_id: &str) -> Result<String, restson::Error> {
            Ok(format!("v1/budgets/{}", budget_id))
        }
    }

    impl RestPath<(&str, &str)> for Wrapper<AccountResponse> {
        fn get_path(params: (&str, &str)) -> Result<String, restson::Error> {
            let (budget_id, account_id) = params;
            Ok(format!("v1/budgets/{}/accounts/{}", budget_id, account_id))
        }
    }

    impl RestPath<&str> for Wrapper<AccountsResponse> {
        fn get_path(budget_id: &str) -> Result<String, restson::Error> {
            Ok(format!("v1/budgets/{}/accounts", budget_id))
        }
    }

    impl RestPath<&str> for Wrapper<PayeesResponse> {
        fn get_path(budget_id: &str) -> Result<String, restson::Error> {
            Ok(format!("v1/budgets/{}/payees", budget_id))
        }
    }

    impl RestPath<&str> for NewTransactions {
        fn get_path(budget_id: &str) -> Result<String, restson::Error> {
            Ok(format!("v1/budgets/{}/transactions", budget_id))
        }
    }

    #[derive(Deserialize)]
    pub struct Wrapper<T> {
        pub data: T,
    }
}
