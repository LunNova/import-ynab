# Import YNAB

Import transactions from truelayer (indirect API to many UK banks) or revolut (personal account API, unofficial).

Multiple currencies are handled by converting at the time of transaction, and reconciling final balance whenever sync is ran.

## Build

`cargo build`

## Config

`./import-ynab config init` will create default config files in the secrets/ directory.

### Revolut account

To add a revolut account, run `./import-ynab config add-revolut` and follow the steps to log in.
This matches the authentication flow used by the app, you will need access to the phone number to receive a verification code.
Note that the secret retrieved here has full access to your revolut account, including the ability to make payments.
Truelayer can be used instead for read only access, but will need renewed every 90 days.

### Truelayer account

For truelayer usage, you must sign up for a free personal account with truelayer and enter a truelayer client secret in the config file.

To add a revolut account, run `./import-ynab config add-truelayer` and follow the steps to log in.
This is an oauth flow. You will visit the URL, authenticate with a bank through truelayer, then be given a code to enter in.
The code will be used to get a permanent (or 90 day) grant with access to view account details.

## Sync

Run `./import-ynab sync` to sync. YNAB accounts are matched to bank accounts by IDs, which must be entered in the notes field in YNAB.
Copy these from the sync log.