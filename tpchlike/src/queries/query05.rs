use timely::order::TotalOrder;
use timely::dataflow::*;
use timely::dataflow::operators::probe::Handle as ProbeHandle;

use differential_dataflow::operators::*;
use differential_dataflow::lattice::Lattice;

use ::Collections;
use ::types::create_date;

// -- $ID$
// -- TPC-H/TPC-R Local Supplier Volume Query (Q5)
// -- Functional Query Definition
// -- Approved February 1998
// :x
// :o
// select
//     n_name,
//     sum(l_extendedprice * (1 - l_discount)) as revenue
// from
//     customer,
//     orders,
//     lineitem,
//     supplier,
//     nation,
//     region
// where
//     c_custkey = o_custkey
//     and l_orderkey = o_orderkey
//     and l_suppkey = s_suppkey
//     and c_nationkey = s_nationkey
//     and s_nationkey = n_nationkey
//     and n_regionkey = r_regionkey
//     and r_name = ':1'
//     and o_orderdate >= date ':2'
//     and o_orderdate < date ':2' + interval '1' year
// group by
//     n_name
// order by
//     revenue desc;
// :n -1

fn starts_with(source: &[u8], query: &[u8]) -> bool {
    source.len() >= query.len() && &source[..query.len()] == query
}

pub fn query<G: Scope>(collections: &mut Collections<G>) -> ProbeHandle<G::Timestamp> 
where G::Timestamp: Lattice+TotalOrder+Ord {

    let regions = 
    collections
        .regions()
        .filter(|x| starts_with(&x.name[..], b"ASIA"))
        .map(|x| x.region_key);

    let nations = 
    collections
        .nations()
        .map(|x| (x.region_key, x.nation_key))
        .semijoin(&regions)
        .map(|(_region_key, nation_key)| nation_key);

    let suppliers = 
    collections
        .suppliers()
        .map(|x| (x.nation_key, x.supp_key))
        .semijoin(&nations)
        .map(|(_nat, supp)| (supp, _nat));

    let customers = 
    collections
        .customers()
        .map(|c| (c.nation_key, c.cust_key))
        .semijoin(&nations)
        .map(|c| (c.1, c.0));
        
    let orders =
    collections
        .orders()
        .flat_map(|o| 
            if o.order_date >= create_date(1994, 1, 1) && o.order_date < create_date(1995, 1, 1) { 
                Some((o.cust_key, o.order_key)) 
            } 
            else { None }
        )
        .join(&customers)
        .map(|o| (o.1, o.2));

    let lineitems = collections
        .lineitems()
        .explode(|l| Some(((l.order_key, l.supp_key), (l.extended_price * (100 - l.discount) / 100) as isize)))
        .join(&orders)
        .map(|(_order, supp, nat)| (supp, nat));

    suppliers
        .map(|x| (x, ()))
        .semijoin(&lineitems)
        .map(|((_supp, nat), ())| nat)
        .count_total()
        // .inspect(|x| println!("{:?}", x))
        .probe()
}