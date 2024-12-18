mod helpers;
mod ibkr;
mod logging;
mod orders;
mod structs;

use std::{
    fs::File,
    process::exit,
    thread::sleep,
    time::{Duration, Instant},
};

use chrono::Utc;
use helpers::{
    calc_final_num_orders, format_strike, get_arb_value, get_discount_value, get_dotenv_variable,
    get_fill_type, get_mode, get_num_days, get_num_days_offset, get_option, get_seconds_to_sleep,
    get_strike_dif_value, get_ticker, is_us_stock_market_open,
};
use ibkr::IBKR;
use logging::{log_error, log_message};

/// Entry point of the bot application.
///
/// This function initializes the bot, connects to the IBKR system, and enters a loop where it checks
/// if the market is open, calculates the number of orders and fills, retrieves contender contracts,
/// and submits orders based on the current market conditions.
///
/// The loop continues to run, sleeping for a specified duration between each iteration, until the
/// market is closed or there are no trades to make.
///
/// # Workflow:
///
/// 1. Initializes the log file.
/// 2. Retrieves necessary data (ticker, options, fill type, etc.) from environment variables or user input.
/// 3. Initializes the IBKR connection.
/// 4. Enters a loop that performs market checks, calculates orders, submits trades, and logs results.
/// 5. Handles errors and logs them appropriately.
///
/// # Example
///
/// ```
/// fn main() {
///     main();
/// }
/// ```
fn main() {
    let _ = File::create("log.txt");
    let mut num_orders: i32;
    let mut num_fills: i32;
    let mut port_val: f64;
    let mut ibkr: IBKR = IBKR::new();

    let ticker: String = get_ticker();
    let option: String = get_option();
    let fill: String = get_fill_type();
    let mode: bool = get_mode();
    let seconds_to_sleep: u64 = get_seconds_to_sleep();

    match ibkr.init(
        ticker,
        get_discount_value(),
        get_arb_value(),
        get_strike_dif_value(),
        match get_dotenv_variable("DOMAIN") {
            Ok(val) => val,
            Err(_) => "localhost".to_string(),
        },
        match get_dotenv_variable("PORT") {
            Ok(val) => val,
            Err(_) => "5000".to_string(),
        },
        get_num_days(),
        get_num_days_offset(),
    ) {
        Ok(_) => log_message(format!("Bot is live.")),
        Err(e) => log_error(format!("{}", e)),
    }

    loop {
        if !mode || is_us_stock_market_open(Utc::now()) {
            if !mode {
                port_val = 100000.0;
            } else {
                match ibkr.get_portfolio_value() {
                    Ok(port_value) => {
                        port_val = port_value;
                    }
                    Err(e) => {
                        log_error(format!("{}", e));
                        exit(1);
                    }
                }
            }

            (num_orders, num_fills) = calc_final_num_orders(&fill, port_val);

            if num_orders > 0 {
                let start_time: Instant = Instant::now();
                let mut end_time: Option<Duration> = None;

                match ibkr.get_contender_contracts(&option, num_orders) {
                    Ok(contender_contracts) => {
                        if !contender_contracts.is_empty() {
                            if mode {
                                match ibkr
                                    .order_contender_contracts(&contender_contracts, num_fills)
                                {
                                    Ok(_) => log_message(format!("Ordering Contracts...")),
                                    Err(e) => log_error(format!("{}", e)),
                                }
                            }
                            end_time = Some(start_time.elapsed());
                            for contender in contender_contracts {
                                log_message(format!(
                                    "Submitting Order for {} * {} {} @ {:.2}:",
                                    num_fills,
                                    contender.type_spread,
                                    contender.exp_date,
                                    contender.arb_val
                                ));

                                for i in 0..contender.contracts.len() {
                                    log_message(format!(
                                        "\tLeg {}: {} {} * {}{} {} @ {:.2}",
                                        i + 1,
                                        contender.action(i),
                                        contender.multiplier(num_fills, i),
                                        format_strike(contender.contracts[i].strike),
                                        contender.contracts[i].type_contract,
                                        contender.contracts[i].date,
                                        contender.contracts[i].mkt_price
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => log_error(format!("{}", e)),
                }

                // Record the current time after running the program.
                if let Some(duration) = end_time {
                    log_message(format!("Total time taken: {:?}.", duration));
                }
            } else {
                log_message(format!("Not enough equity in account to make a trade."));
                break;
            }

            // Sleep to avoid throttling resources.
            log_message(format!(""));
            log_message(format!("Sleeping for {} seconds.", seconds_to_sleep));
            sleep(Duration::from_secs(seconds_to_sleep));
            log_message(format!("Awake after {} seconds.", seconds_to_sleep));
            log_message(format!(""));

            if mode {
                ibkr.cancel_pending_orders();
            }
            sleep(Duration::from_secs(5));
        } else {
            log_message(format!("Market is closed."));
            break;
        }
    }
    log_message(format!("Exiting..."));
}
