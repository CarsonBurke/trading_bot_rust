#[allow(dead_code)]
mod helpers;
#[allow(dead_code)]
mod structs;

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, env, error::Error};

    use chrono::{Datelike, LocalResult, TimeZone, Utc, Weekday};
    use ordered_float::OrderedFloat;

    use crate::{
        helpers::{
            build_boxspread_order, build_butterfly_order, build_calendar_order,
            calc_final_num_orders, calc_rank_value, calc_time_difference, convert_date,
            generate_conids_structure, generate_months_slice, get_boxspread_contenders,
            get_butterfly_contenders, get_calendar_contenders, get_dotenv_variable,
            is_us_stock_market_open, is_weekday,
        },
        structs::{Contender, Contract, Opt},
    };

    #[test]
    fn test_get_dotenv_variable() {
        // Mock the environment variable.
        env::set_var("TEST_KEY", "test_value");

        // Check if the function retrieves the value correctly.
        let result: Result<String, Box<dyn Error>> = get_dotenv_variable("TEST_KEY");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_value".to_string());

        // Clean up (optional but good practice).
        env::remove_var("TEST_KEY");

        // Test with a non-existent key.
        let result: Result<String, Box<dyn Error>> = get_dotenv_variable("NON_EXISTENT_KEY");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_us_stock_market_open() {
        // Function to safely extract DateTime from LocalResult.
        fn extract_datetime(result: LocalResult<chrono::DateTime<Utc>>) -> chrono::DateTime<Utc> {
            match result {
                LocalResult::Single(dt) => dt,
                _ => panic!("Invalid date/time provided."),
            }
        }

        // Test when market is definitely closed.
        let time: chrono::DateTime<Utc> =
            extract_datetime(Utc.with_ymd_and_hms(2023, 10, 10, 5, 0, 0)); // 5:00 AM UTC.
        assert_eq!(is_us_stock_market_open(time), false);

        // Test right at market open.
        let time: chrono::DateTime<Utc> =
            extract_datetime(Utc.with_ymd_and_hms(2023, 10, 10, 9, 30, 0)); // 9:30 AM UTC.
        assert_eq!(is_us_stock_market_open(time), true);

        // Test during open market hours.
        let time: chrono::DateTime<Utc> =
            extract_datetime(Utc.with_ymd_and_hms(2023, 10, 10, 12, 0, 0)); // 12:00 PM UTC.
        assert_eq!(is_us_stock_market_open(time), true);

        // Test right at market close.
        let time: chrono::DateTime<Utc> =
            extract_datetime(Utc.with_ymd_and_hms(2023, 10, 10, 15, 15, 0)); // 3:15 PM UTC.
        assert_eq!(is_us_stock_market_open(time), true);

        // Test right after market close.
        let time: chrono::DateTime<Utc> =
            extract_datetime(Utc.with_ymd_and_hms(2023, 10, 10, 15, 16, 0)); // 3:16 PM UTC.
        assert_eq!(is_us_stock_market_open(time), false);
    }

    #[test]
    fn test_is_weekday() {
        // Obtain the current day of the week.
        let today: Weekday = Utc::now().weekday();

        // Determine if today is a weekday. If today is Saturday or Sunday, it should return false; otherwise, true.
        let expected: bool = today != Weekday::Sat && today != Weekday::Sun;

        // Check that the function's result matches the expected value.
        assert_eq!(is_weekday(), expected);
    }

    #[test]
    fn test_calc_final_num_orders() {
        // Test for port_val less than 600.
        assert_eq!(calc_final_num_orders("1", 599.0), (0, 0));

        // Test for fill type "1".
        assert_eq!(calc_final_num_orders("1", 600.0), (1, 1));
        assert_eq!(calc_final_num_orders("1", 1200.0), (1, 1));

        // Test for fill type "2".
        assert_eq!(calc_final_num_orders("2", 600.0), (1, 1));
        assert_eq!(calc_final_num_orders("2", 1200.0), (1, 2));

        // Test for fill type "3".
        assert_eq!(calc_final_num_orders("3", 600.0), (1, 1));
        assert_eq!(calc_final_num_orders("3", 1200.0), (2, 1));
    }

    #[test]
    fn test_calc_time_difference() {
        // Test with a difference of 1 day.
        // Current date: 220101, Date: 220102, Expected difference: 1 day.
        let difference: i64 = calc_time_difference("220101", "220102");
        assert_eq!(difference, 1);

        // Test with a difference of 5 days.
        // Current date: 220101, Date: 220106, Expected difference: 5 days.
        let difference: i64 = calc_time_difference("220101", "220106");
        assert_eq!(difference, 5);

        // Test with dates being the same.
        // Current date: 220101, Date: 220101, Expected difference: 0 days.
        let difference: i64 = calc_time_difference("220101", "220101");
        assert_eq!(difference, 0);

        // Test with the current date being later than the date.
        // Current date: 220106, Date: 220101, Expected difference: -5 days.
        let difference: i64 = calc_time_difference("220106", "220101");
        assert_eq!(difference, -5);
    }

    #[test]
    fn test_calc_rank_value() {
        // Test with a time difference of 1 day.
        // Current date: 220101, Date: 220102, avg_ask: 10.0, arb_val: 5.0, Expected rank value: 50.0.
        let rank_value: f64 = calc_rank_value(10.0, 5.0, "220101", "220102");
        assert!((rank_value - (50.0 / 2.0)).abs() < 1e-9); // Using a small epsilon for floating point comparison.

        // Test with a time difference of 5 days.
        // Current date: 220101, Date: 220106, avg_ask: 10.0, arb_val: 5.0, Expected rank value: 12.5.
        let rank_value: f64 = calc_rank_value(10.0, 5.0, "220101", "220106");
        assert!((rank_value - (50.0 / 6.0)).abs() < 1e-9);

        // Test with dates being the same.
        // Current date: 220101, Date: 220101, avg_ask: 10.0, arb_val: 5.0, Expected rank value: 500.0.
        let rank_value: f64 = calc_rank_value(10.0, 5.0, "220101", "220101");
        assert!((rank_value - (50.0 / 1.0)).abs() < 1e-9);
    }

    #[test]
    fn test_convert_date() {
        // Test date conversion for January of 2022.
        // Input date: "220101", Expected converted date: "JAN22".
        let converted_date: String = convert_date("220101");
        assert_eq!(converted_date, "JAN22");

        // Test date conversion for December of 2022.
        // Input date: "221231", Expected converted date: "DEC22".
        let converted_date: String = convert_date("221231");
        assert_eq!(converted_date, "DEC22");

        // Test date conversion for May of 2022.
        // Input date: "220515", Expected converted date: "MAY22".
        let converted_date: String = convert_date("220515");
        assert_eq!(converted_date, "MAY22");

        // Test date conversion for a leap year, February of 2024.
        // Input date: "240229", Expected converted date: "FEB24".
        let converted_date: String = convert_date("240229");
        assert_eq!(converted_date, "FEB24");
    }

    #[test]
    fn test_generate_conids_structure() {
        // Mock data setup for the test.
        let dates_slice: Vec<String> = vec!["2021-11-01".to_string(), "2021-11-02".to_string()];

        let mut strike_data_for_date1: HashMap<String, Vec<f64>> = HashMap::new();
        strike_data_for_date1.insert("C".to_string(), vec![3000.0, 3100.0]);
        strike_data_for_date1.insert("P".to_string(), vec![2900.0, 2800.0]);

        let mut strike_data_for_date2: HashMap<String, Vec<f64>> = HashMap::new();
        strike_data_for_date2.insert("C".to_string(), vec![3050.0, 3150.0]);
        strike_data_for_date2.insert("P".to_string(), vec![2950.0, 2850.0]);

        let mut strike_slice: HashMap<String, HashMap<String, Vec<f64>>> = HashMap::new();
        strike_slice.insert("2021-11-01".to_string(), strike_data_for_date1);
        strike_slice.insert("2021-11-02".to_string(), strike_data_for_date2);

        // Call the function with the mock data.
        let result: HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, String>>> =
            generate_conids_structure(&dates_slice, &strike_slice);

        // Assertions to verify the structure of the result.
        assert!(result.contains_key("2021-11-01"));
        assert!(result["2021-11-01"].contains_key("C"));
        assert!(result["2021-11-01"]["C"].contains_key(&OrderedFloat(3000.0)));
        assert!(result["2021-11-01"]["C"].contains_key(&OrderedFloat(3100.0)));
        assert!(result["2021-11-01"].contains_key("P"));
        assert!(result["2021-11-01"]["P"].contains_key(&OrderedFloat(2900.0)));
        assert!(result["2021-11-01"]["P"].contains_key(&OrderedFloat(2800.0)));

        assert!(result.contains_key("2021-11-02"));
        assert!(result["2021-11-02"].contains_key("C"));
        assert!(result["2021-11-02"]["C"].contains_key(&OrderedFloat(3050.0)));
        assert!(result["2021-11-02"]["C"].contains_key(&OrderedFloat(3150.0)));
        assert!(result["2021-11-02"].contains_key("P"));
        assert!(result["2021-11-02"]["P"].contains_key(&OrderedFloat(2950.0)));
        assert!(result["2021-11-02"]["P"].contains_key(&OrderedFloat(2850.0)));
    }

    #[test]
    fn test_generate_months_slice() {
        // Mock data setup for the test.
        let dates_slice: Vec<String> = vec![
            "211101".to_string(),
            "211102".to_string(),
            "211201".to_string(),
            "211202".to_string(),
            "211101".to_string(), // Intentionally adding duplicate for uniqueness check.
        ];

        // Call the function with the mock data.
        let result: Vec<String> = generate_months_slice(&dates_slice);

        // Assertions to verify the structure of the result.
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "NOV21");
        assert_eq!(result[1], "DEC21");
    }

    #[test]
    fn test_build_calendar_order() {
        // Mock data setup for the test.
        let contract: Contender = Contender {
            type_spread: "Calendar".to_string(), // Specify the spread type for the contender.
            arb_val: 1.0,                        // Arbitrage value for the contender.
            contracts: vec![
                Contract {
                    date: "2021-11-01".to_string(),
                    type_contract: "C".to_string(),
                    strike: *OrderedFloat(3000.0),
                    mkt_price: 12.2,
                },
                Contract {
                    date: "2021-11-02".to_string(),
                    type_contract: "C".to_string(),
                    strike: *OrderedFloat(3100.0),
                    mkt_price: 11.2,
                },
            ],
            avg_ask: 3.5,
            exp_date: "2021-11-01".to_string(),
            rank_value: 1.75,
        };

        // Creating a nested map to mock the `conids_map` for the test.
        let mut inner_map_1: HashMap<String, HashMap<OrderedFloat<f64>, String>> = HashMap::new();
        inner_map_1.insert("C".to_string(), {
            let mut strike_map: HashMap<OrderedFloat<f64>, String> = HashMap::new();
            strike_map.insert(OrderedFloat(3000.0), "CONID1".to_string()); // Mapping a strike price to a contract ID.
            strike_map
        });

        let mut inner_map_2: HashMap<String, HashMap<OrderedFloat<f64>, String>> = HashMap::new();
        inner_map_2.insert("C".to_string(), {
            let mut strike_map: HashMap<OrderedFloat<f64>, String> = HashMap::new();
            strike_map.insert(OrderedFloat(3100.0), "CONID2".to_string()); // Mapping another strike price to a contract ID.
            strike_map
        });

        let mut conids_map: HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, String>>> =
            HashMap::new();
        conids_map.insert("2021-11-01".to_string(), inner_map_1);
        conids_map.insert("2021-11-02".to_string(), inner_map_2);

        // Call the function with the mock data to obtain a result.
        let result: crate::structs::OrderBody = build_calendar_order(
            &contract,
            2,
            &Some("ACCOUNT_ID".to_string()),
            &Some(conids_map),
            Some(0.9),
        );

        // Assertions to verify the correctness of the output.
        assert_eq!(result.acct_id, "ACCOUNT_ID"); // Ensure the account ID matches the expected value.
        assert_eq!(result.con_idex, "28812380;;;CONID1/-1,CONID2/1"); // Verify the construction of the contract index string.
        assert_eq!(result.price, -0.9); // Validate the calculated price.
        assert_eq!(result.quantity, 2); // Check the order quantity.
    }

    #[test]
    fn test_build_butterfly_order() {
        // Mock data setup for the test.
        let contract: Contender = Contender {
            type_spread: "Butterfly".to_string(), // Specify the spread type for the contender.
            arb_val: 2.0,                         // Arbitrage value for the contender.
            contracts: vec![
                Contract {
                    date: "2021-11-01".to_string(),
                    type_contract: "C".to_string(),
                    strike: *OrderedFloat(2900.0),
                    mkt_price: 10.2,
                },
                Contract {
                    date: "2021-11-02".to_string(),
                    type_contract: "C".to_string(),
                    strike: *OrderedFloat(3000.0),
                    mkt_price: 11.2,
                },
                Contract {
                    date: "2021-11-03".to_string(),
                    type_contract: "C".to_string(),
                    strike: *OrderedFloat(3100.0),
                    mkt_price: 12.2,
                },
            ],
            avg_ask: 4.0,
            exp_date: "2021-11-03".to_string(),
            rank_value: 2.5,
        };

        // Creating a nested map to mock the `conids_map` for the test.
        let mut inner_map_1: HashMap<String, HashMap<OrderedFloat<f64>, String>> = HashMap::new();
        inner_map_1.insert("C".to_string(), {
            let mut strike_map: HashMap<OrderedFloat<f64>, String> = HashMap::new();
            strike_map.insert(OrderedFloat(2900.0), "CONID1".to_string()); // Mapping a strike price to a contract ID.
            strike_map
        });

        let mut inner_map_2: HashMap<String, HashMap<OrderedFloat<f64>, String>> = HashMap::new();
        inner_map_2.insert("C".to_string(), {
            let mut strike_map: HashMap<OrderedFloat<f64>, String> = HashMap::new();
            strike_map.insert(OrderedFloat(3000.0), "CONID2".to_string()); // Mapping another strike price to a contract ID.
            strike_map
        });

        let mut inner_map_3: HashMap<String, HashMap<OrderedFloat<f64>, String>> = HashMap::new();
        inner_map_3.insert("C".to_string(), {
            let mut strike_map: HashMap<OrderedFloat<f64>, String> = HashMap::new();
            strike_map.insert(OrderedFloat(3100.0), "CONID3".to_string()); // Mapping yet another strike price to a contract ID.
            strike_map
        });

        let mut conids_map: HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, String>>> =
            HashMap::new();
        conids_map.insert("2021-11-01".to_string(), inner_map_1);
        conids_map.insert("2021-11-02".to_string(), inner_map_2);
        conids_map.insert("2021-11-03".to_string(), inner_map_3);

        // Call the function with the mock data to obtain a result.
        let result: crate::structs::OrderBody = build_butterfly_order(
            &contract,
            3,
            &Some("ACCOUNT_ID".to_string()),
            &Some(conids_map),
            Some(0.95),
        );

        // Assertions to verify the correctness of the output.
        assert_eq!(result.acct_id, "ACCOUNT_ID"); // Ensure the account ID matches the expected value.
        assert_eq!(result.con_idex, "28812380;;;CONID2/-2,CONID1/1,CONID3/1"); // Verify the construction of the contract index string.
        assert_eq!(result.price, -1.9); // Validate the calculated price.
        assert_eq!(result.quantity, 3); // Check the order quantity.
    }

    #[test]
    fn test_build_boxspread_order() {
        // Mock data setup for the test.
        let contract: Contender = Contender {
            type_spread: "BoxSpread".to_string(), // Specify the spread type for the contender.
            arb_val: 2.5,                         // Arbitrage value for the contender.
            contracts: vec![
                Contract {
                    date: "2021-11-01".to_string(),
                    type_contract: "C".to_string(),
                    strike: *OrderedFloat(2800.0),
                    mkt_price: 9.2,
                },
                Contract {
                    date: "2021-11-02".to_string(),
                    type_contract: "C".to_string(),
                    strike: *OrderedFloat(2900.0),
                    mkt_price: 10.2,
                },
                Contract {
                    date: "2021-11-03".to_string(),
                    type_contract: "P".to_string(),
                    strike: *OrderedFloat(2800.0),
                    mkt_price: 11.2,
                },
                Contract {
                    date: "2021-11-04".to_string(),
                    type_contract: "P".to_string(),
                    strike: *OrderedFloat(2900.0),
                    mkt_price: 12.2,
                },
            ],
            avg_ask: 5.0,
            exp_date: "2021-11-04".to_string(),
            rank_value: 3.0,
        };

        // Creating a nested map to mock the `conids_map` for the test.
        let mut inner_map_call_1: HashMap<String, HashMap<OrderedFloat<f64>, String>> =
            HashMap::new();
        inner_map_call_1.insert("C".to_string(), {
            let mut strike_map: HashMap<OrderedFloat<f64>, String> = HashMap::new();
            strike_map.insert(OrderedFloat(2800.0), "CONID1".to_string()); // Mapping a strike price to a contract ID.
            strike_map
        });
        let mut inner_map_call_2: HashMap<String, HashMap<OrderedFloat<f64>, String>> =
            HashMap::new();
        inner_map_call_2.insert("C".to_string(), {
            let mut strike_map: HashMap<OrderedFloat<f64>, String> = HashMap::new();
            strike_map.insert(OrderedFloat(2900.0), "CONID2".to_string()); // Mapping another strike price to a contract ID.
            strike_map
        });
        let mut inner_map_put_1: HashMap<String, HashMap<OrderedFloat<f64>, String>> =
            HashMap::new();
        inner_map_put_1.insert("P".to_string(), {
            let mut strike_map: HashMap<OrderedFloat<f64>, String> = HashMap::new();
            strike_map.insert(OrderedFloat(2800.0), "CONID3".to_string()); // Mapping yet another strike price to a contract ID.
            strike_map
        });
        let mut inner_map_put_2: HashMap<String, HashMap<OrderedFloat<f64>, String>> =
            HashMap::new();
        inner_map_put_2.insert("P".to_string(), {
            let mut strike_map: HashMap<OrderedFloat<f64>, String> = HashMap::new();
            strike_map.insert(OrderedFloat(2900.0), "CONID4".to_string()); // Mapping the last strike price to a contract ID.
            strike_map
        });

        let mut conids_map: HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, String>>> =
            HashMap::new();
        conids_map.insert("2021-11-01".to_string(), inner_map_call_1);
        conids_map.insert("2021-11-02".to_string(), inner_map_call_2);
        conids_map.insert("2021-11-03".to_string(), inner_map_put_1);
        conids_map.insert("2021-11-04".to_string(), inner_map_put_2);

        // Call the function with the mock data to obtain a result.
        let result: crate::structs::OrderBody = build_boxspread_order(
            &contract,
            4,
            &Some("ACCOUNT_ID".to_string()),
            &Some(conids_map),
            Some(0.9),
        );

        // Assertions to verify the correctness of the output.
        assert_eq!(result.acct_id, "ACCOUNT_ID"); // Ensure the account ID matches the expected value.
        assert_eq!(
            result.con_idex,
            "28812380;;;CONID4/-1,CONID3/1,CONID1/1,CONID2/-1"
        ); // Verify the construction of the contract index string.
        assert_eq!(result.price, -2.25); // Validate the calculated price.
        assert_eq!(result.quantity, 4); // Check the order quantity.
    }

    #[test]
    fn test_get_calendar_contenders() {
        // Mock data setup for the test.
        let dates_slice: Vec<String> = vec!["210101".to_string(), "210102".to_string()];

        let mut strike_slice: HashMap<String, HashMap<String, Vec<f64>>> = HashMap::new();
        strike_slice.insert("210101".to_string(), {
            let mut map: HashMap<String, Vec<f64>> = HashMap::new();
            map.insert("C".to_string(), vec![100.0]);
            map
        });
        strike_slice.insert("210102".to_string(), {
            let mut map: HashMap<String, Vec<f64>> = HashMap::new();
            map.insert("C".to_string(), vec![100.0]);
            map
        });

        let mut contracts_map: HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, Opt>>> =
            HashMap::new();
        contracts_map.insert("210101".to_string(), {
            let mut map: HashMap<String, HashMap<OrderedFloat<f64>, Opt>> = HashMap::new();
            map.insert("C".to_string(), {
                let mut inner_map: HashMap<OrderedFloat<f64>, Opt> = HashMap::new();
                inner_map.insert(
                    OrderedFloat(100.0),
                    Opt {
                        mkt: 2.2,
                        bid: 1.3,
                        asz: 8.0,
                    },
                );
                inner_map
            });
            map
        });
        contracts_map.insert("210102".to_string(), {
            let mut map: HashMap<String, HashMap<OrderedFloat<f64>, Opt>> = HashMap::new();
            map.insert("C".to_string(), {
                let mut inner_map: HashMap<OrderedFloat<f64>, Opt> = HashMap::new();
                inner_map.insert(
                    OrderedFloat(100.0),
                    Opt {
                        mkt: 1.9,
                        bid: 1.3,
                        asz: 12.0,
                    },
                );
                inner_map
            });
            map
        });

        // Call the function with the mock data.
        let result: Vec<Contender> =
            get_calendar_contenders(&contracts_map, &dates_slice, &strike_slice).unwrap();

        // Assertions.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].arb_val, 0.3);
        assert_eq!(result[0].avg_ask, 10.0);
        assert_eq!(result[0].type_spread, "Calendar");
        assert_eq!(result[0].exp_date, "210101");
    }

    #[test]
    fn test_get_butterfly_contenders() {
        // Mock data setup for the test.
        let dates_slice: Vec<String> = vec!["210101".to_string()];

        let mut strike_slice: HashMap<String, HashMap<String, Vec<f64>>> = HashMap::new();
        strike_slice.insert("210101".to_string(), {
            let mut map: HashMap<String, Vec<f64>> = HashMap::new();
            map.insert("C".to_string(), vec![95.0, 100.0, 105.0]);
            map
        });

        let mut contracts_map: HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, Opt>>> =
            HashMap::new();
        contracts_map.insert("210101".to_string(), {
            let mut map: HashMap<String, HashMap<OrderedFloat<f64>, Opt>> = HashMap::new();
            map.insert("C".to_string(), {
                let mut inner_map: HashMap<OrderedFloat<f64>, Opt> = HashMap::new();
                inner_map.insert(
                    OrderedFloat(95.0),
                    Opt {
                        mkt: 2.1,
                        bid: 1.3,
                        asz: 8.0,
                    },
                );
                inner_map.insert(
                    OrderedFloat(100.0),
                    Opt {
                        mkt: 2.3,
                        bid: 1.3,
                        asz: 10.0,
                    },
                );
                inner_map.insert(
                    OrderedFloat(105.0),
                    Opt {
                        mkt: 2.2,
                        bid: 1.3,
                        asz: 12.0,
                    },
                );
                inner_map
            });
            map
        });

        // Call the function with the mock data.
        let result: Vec<Contender> =
            get_butterfly_contenders(&contracts_map, &dates_slice, &strike_slice).unwrap();

        // Assertions.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].arb_val, 0.3); // 2 * 2.3 (center) - (2.1 (left) + 2.2 (right)).
        assert_eq!(result[0].avg_ask, 10.0);
        assert_eq!(result[0].type_spread, "Butterfly");
        assert_eq!(result[0].exp_date, "210101");
    }

    #[test]
    fn test_get_boxspread_contenders() {
        // Mock data setup for the test.
        let dates_slice: Vec<String> = vec!["210101".to_string()];

        let mut strike_slice: HashMap<String, HashMap<String, Vec<f64>>> = HashMap::new();
        strike_slice.insert("210101".to_string(), {
            let mut map: HashMap<String, Vec<f64>> = HashMap::new();
            map.insert("C".to_string(), vec![95.0, 100.0]);
            map.insert("P".to_string(), vec![95.0, 100.0]);
            map
        });

        let mut contracts_map: HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, Opt>>> =
            HashMap::new();
        contracts_map.insert("210101".to_string(), {
            let mut map: HashMap<String, HashMap<OrderedFloat<f64>, Opt>> = HashMap::new();
            map.insert("C".to_string(), {
                let mut inner_map: HashMap<OrderedFloat<f64>, Opt> = HashMap::new();
                inner_map.insert(
                    OrderedFloat(95.0),
                    Opt {
                        mkt: 1.4,
                        bid: 1.3,
                        asz: 8.0,
                    },
                );
                inner_map.insert(
                    OrderedFloat(100.0),
                    Opt {
                        mkt: 2.1,
                        bid: 1.3,
                        asz: 12.0,
                    },
                );
                inner_map
            });
            map.insert("P".to_string(), {
                let mut inner_map: HashMap<OrderedFloat<f64>, Opt> = HashMap::new();
                inner_map.insert(
                    OrderedFloat(95.0),
                    Opt {
                        mkt: 1.5,
                        bid: 1.3,
                        asz: 12.0,
                    },
                );
                inner_map.insert(
                    OrderedFloat(100.0),
                    Opt {
                        mkt: 2.5,
                        bid: 1.3,
                        asz: 8.0,
                    },
                );
                inner_map
            });
            map
        });

        // Call the function with the mock data.
        let result: Vec<Contender> =
            get_boxspread_contenders(&contracts_map, &dates_slice, &strike_slice).unwrap();

        // Assertions.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].arb_val, 0.3); // (1.4 + 2.5) - (1.5 + 2.1).
        assert_eq!(result[0].avg_ask, 10.0);
        assert_eq!(result[0].type_spread, "Boxspread");
        assert_eq!(result[0].exp_date, "210101");
    }
}
