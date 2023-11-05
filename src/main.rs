mod activetick;
mod helpers;
mod ibkr;
mod structs;

use std::{
    fs::File,
    process::exit,
    thread::sleep,
    time::{Duration, Instant},
};

use activetick::ActiveTick;
use chrono::Utc;
use helpers::{
    calc_final_num_orders, get_api_key, get_discount_value, get_dotenv_variable, get_fill_type,
    get_mode, get_option, get_password, get_seconds_to_sleep, get_username,
    is_us_stock_market_open, is_weekday, log_error, log_message,
};
use ibkr::IBKR;

fn main() {
    let _ = File::create("log.txt");
    let mut num_orders: i32;
    let mut num_fills: i32;
    let mut port_val: f64;
    let mut active_tick: ActiveTick = ActiveTick::new();
    let mut ibkr: IBKR = IBKR::new();

    match active_tick.init(&get_username(), &get_password(), &get_api_key(), 4) {
        Ok(_) => log_message(format!("User authenticated.")),
        Err(e) => log_error(format!("{}", e)),
    }

    let option: String = get_option();
    let fill: String = get_fill_type();
    let mode: bool = get_mode();
    let seconds_to_sleep: u64 = get_seconds_to_sleep();

    if mode {
        match ibkr.init(
            get_discount_value(),
            match get_dotenv_variable("DOMAIN") {
                Ok(val) => val,
                Err(_) => "localhost".to_string(),
            },
            match get_dotenv_variable("PORT") {
                Ok(val) => val,
                Err(_) => "5000".to_string(),
            },
            active_tick.get_dates_slice(),
            active_tick.get_strike_slice(),
        ) {
            Ok(_) => log_message(format!("Bot is live.")),
            Err(e) => log_error(format!("{}", e)),
        }
    }

    loop {
        if is_us_stock_market_open(Utc::now()) && is_weekday() || !mode {
            if mode {
                match ibkr.get_portfolio_value() {
                    Ok(portfolio_val) => {
                        port_val = portfolio_val;
                    }
                    Err(e) => {
                        log_error(format!("{}", e));
                        exit(1);
                    }
                }
            } else {
                port_val = 100000.0;
            }

            (num_orders, num_fills) = calc_final_num_orders(&fill, port_val);

            if num_orders > 0 {
                let start_time: Instant = Instant::now();
                let mut end_time: Option<Duration> = None;

                match active_tick.get_contender_contracts(&option, num_orders) {
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
                                        "\tLeg {}: {} {} * {:.2}{} {} @ {:.2}",
                                        i + 1,
                                        contender.action(i),
                                        contender.multiplier(num_fills, i),
                                        contender.contracts[i].strike as i64,
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
        } else {
            log_message(format!("Market is closed."));
            break;
        }
    }
    log_message(format!("Exiting..."));
}
