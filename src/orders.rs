use std::collections::HashMap;

use ordered_float::OrderedFloat;

use crate::structs::{Contender, OrderBody, RequestDataStruct};

// Function that builds butterfly order body.
pub(crate) fn build_butterfly_order(
    contract: &Contender,
    num_fills: i32,
    account_id: &Option<String>,
    conids_map: &Option<HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, String>>>>,
    discount_value: Option<f64>,
) -> OrderBody {
    OrderBody {
        acct_id: account_id.clone().unwrap(),
        con_idex: format!(
            "28812380;;;{}/-2,{}/1,{}/1",
            conids_map.as_ref().unwrap()[contract.contracts[1].date.as_str()]
                [contract.contracts[1].type_contract.as_str()]
                [(&contract.contracts[1].strike).into()],
            conids_map.as_ref().unwrap()[contract.contracts[0].date.as_str()]
                [contract.contracts[0].type_contract.as_str()]
                [(&contract.contracts[0].strike).into()],
            conids_map.as_ref().unwrap()[contract.contracts[2].date.as_str()]
                [contract.contracts[2].type_contract.as_str()]
                [(&contract.contracts[2].strike).into()]
        ),
        order_type: "LMT".to_string(),
        listing_exchange: "SMART".to_string(),
        outside_rth: false,
        price: -1.0 * (((contract.arb_val - discount_value.unwrap()) * 100.0).round() / 100.0),
        side: "BUY".to_string(),
        ticker: "SPX".to_string(),
        tif: "DAY".to_string(),
        referrer: "NO_REFERRER_PROVIDED".to_string(),
        quantity: num_fills,
        use_adaptive: false,
    }
}

// Function that builds boxspread order body.
pub(crate) fn build_boxspread_order(
    contract: &Contender,
    num_fills: i32,
    account_id: &Option<String>,
    conids_map: &Option<HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, String>>>>,
    discount_value: Option<f64>,
) -> OrderBody {
    OrderBody {
        acct_id: account_id.clone().unwrap(),
        con_idex: format!(
            "28812380;;;{}/-1,{}/1,{}/1,{}/-1",
            conids_map.as_ref().unwrap()[contract.contracts[3].date.as_str()] // Short right dated Put.
                [contract.contracts[3].type_contract.as_str()]
                [(&contract.contracts[3].strike).into()],
            conids_map.as_ref().unwrap()[contract.contracts[2].date.as_str()] // Long right dated Call.
                [contract.contracts[2].type_contract.as_str()]
                [(&contract.contracts[2].strike).into()],
            conids_map.as_ref().unwrap()[contract.contracts[0].date.as_str()] // Long current dated Put.
                [contract.contracts[0].type_contract.as_str()]
                [(&contract.contracts[0].strike).into()],
            conids_map.as_ref().unwrap()[contract.contracts[1].date.as_str()] // Short current dated Call.
                [contract.contracts[1].type_contract.as_str()]
                [(&contract.contracts[1].strike).into()]
        ),
        order_type: "LMT".to_string(),
        listing_exchange: "SMART".to_string(),
        outside_rth: false,
        price: -1.0 * (((contract.arb_val - discount_value.unwrap()) * 100.0).round() / 100.0),
        side: "BUY".to_string(),
        ticker: "SPX".to_string(),
        tif: "DAY".to_string(),
        referrer: "NO_REFERRER_PROVIDED".to_string(),
        quantity: num_fills,
        use_adaptive: false,
    }
}

// Function that builds request data for json body to submit an order.
pub(crate) fn build_request_data(
    contender_contracts: &Vec<Contender>,
    num_fills: i32,
    account_id: &Option<String>,
    conids_map: &Option<HashMap<String, HashMap<String, HashMap<OrderedFloat<f64>, String>>>>,
    discount_value: Option<f64>,
) -> RequestDataStruct {
    let mut request_data: RequestDataStruct = RequestDataStruct { orders: Vec::new() };

    for contract in contender_contracts {
        match contract.type_spread.as_str() {
            "Butterfly" => {
                request_data.orders.push(build_butterfly_order(
                    contract,
                    num_fills,
                    account_id,
                    conids_map,
                    discount_value,
                ));
            }
            "Boxspread" => {
                request_data.orders.push(build_boxspread_order(
                    contract,
                    num_fills,
                    account_id,
                    conids_map,
                    discount_value,
                ));
            }
            _ => {}
        }
    }

    request_data
}