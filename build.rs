// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use ::anyhow::Result;
use ::bindgen::{Bindings, Builder};
use ::cc::Build;
use ::std::{env, path::Path};

#[cfg(target_os = "windows")]
fn os_build() -> Result<()> {
    use ::std::path::PathBuf;

    let out_dir_s: String = env::var("OUT_DIR")?;
    let out_dir: &Path = Path::new(&out_dir_s);

    let libdpdk_path: String = env::var("LIBDPDK_PATH")?;

    let include_path: String = format!("{}{}", libdpdk_path, "\\include");
    let library_path: String = format!("{}{}", libdpdk_path, "\\lib");

    let libraries: Vec<&str> = vec![
        "rte_cfgfile",
        "rte_hash",
        "rte_cmdline",
        "rte_pci",
        "rte_ethdev",
        "rte_meter",
        "rte_net",
        "rte_mbuf",
        "rte_mempool",
        "rte_rcu",
        "rte_ring",
        "rte_eal",
        "rte_cycles",
        "rte_telemetry",
        "rte_kvargs",
        "rte_memcpy",
        "rte_malloc",
    ];

    let cflags: &str = "-mavx";

    // Step 1: Now that we've compiled and installed DPDK, point cargo to the libraries.
    println!("cargo:rustc-link-search={}", library_path);

    for lib in &libraries {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    }

    // Step 2: Generate bindings for the DPDK headers.
    let bindings: Bindings = Builder::default()
        .clang_arg(&format!("-I{}", include_path))
        .blocklist_type("rte_arp_ipv4")
        .blocklist_type("rte_arp_hdr")
        .blocklist_type("IMAGE_TLS_DIRECTORY")
        .blocklist_type("PIMAGE_TLS_DIRECTORY")
        .blocklist_type("PIMAGE_TLS_DIRECTORY64")
        .blocklist_type("IMAGE_TLS_DIRECTORY64")
        .blocklist_type("_IMAGE_TLS_DIRECTORY64")
        .clang_arg(cflags)
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate_comments(false)
        .generate()?;
    let bindings_out: PathBuf = out_dir.join("bindings.rs");
    bindings.write_to_file(bindings_out)?;

    // Step 3: Compile a stub file so Rust can access `inline` functions in the headers
    // that aren't compiled into the libraries.
    let mut builder: Build = cc::Build::new();
    builder.opt_level(3);
    builder.flag("-march=native");
    builder.file("inlined.c");
    builder.include(include_path);
    builder.compile("inlined");

    Ok(())
}

#[cfg(target_os = "linux")]
fn os_build() -> Result<()> {
    use ::std::process::Command;

    let out_dir_s = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_s);

    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    let cflags_bytes = Command::new("pkg-config")
        .args(&["--cflags", "libdpdk"])
        .output()
        .unwrap_or_else(|e| panic!("Failed pkg-config cflags: {:?}", e))
        .stdout;
    let cflags = String::from_utf8(cflags_bytes).unwrap();

    let mut header_locations = vec![];

    for flag in cflags.split(' ') {
        if flag.starts_with("-I") {
            let header_location = flag[2..].trim();
            header_locations.push(header_location);
        }
    }

    let ldflags_bytes = Command::new("pkg-config")
        .args(&["--libs", "libdpdk"])
        .output()
        .unwrap_or_else(|e| panic!("Failed pkg-config ldflags: {:?}", e))
        .stdout;
    let ldflags = String::from_utf8(ldflags_bytes).unwrap();

    let mut library_location = None;
    let mut lib_names = vec![];

    for flag in ldflags.split(' ') {
        if flag.starts_with("-L") {
            library_location = Some(&flag[2..]);
        } else if flag.starts_with("-l") {
            lib_names.push(&flag[2..]);
        }
    }

    // Link in `librte_net_mlx5` and its dependencies if desired.
    #[cfg(feature = "mlx5")]
    {
        lib_names.extend(&["rte_net_mlx5", "rte_bus_pci", "rte_bus_vdev", "rte_common_mlx5"]);
    }

    // Step 1: Now that we've compiled and installed DPDK, point cargo to the libraries.
    if let Some(location) = library_location {
        println!("cargo:rustc-link-search=native={}", location);
    }

    for lib_name in &lib_names {
        println!("cargo:rustc-link-lib=dylib={}", lib_name);
    }

    // Step 2: Generate bindings for the DPDK headers.
    let mut builder: Builder = Builder::default();
    for header_location in &header_locations {
        builder = builder.clang_arg(&format!("-I{}", header_location));
    }
    let bindings: Bindings = builder
        .allowlist_recursively(true)
        .allowlist_type("rte_mbuf")
        .allowlist_type("rte_mempool")
        .allowlist_type("rte_eth_txconf")
        .allowlist_type("rte_eth_rxconf")
        .allowlist_type("rte_eth_fc_conf")
        .allowlist_type("rte_pktmbuf_pool_private")
        .allowlist_type("rte_ether_addr")
        .allowlist_type("rte_tcp_hdr")
        .allowlist_type("rte_flow_attr")
        .allowlist_type("rte_flow_error")
        .allowlist_type("rte_flow_item")
        .allowlist_type("rte_flow_action")
        .allowlist_type("rte_flow_action_queue")
        .allowlist_type("rte_flow_item_type_RTE_FLOW_ITEM_TYPE_ETH")
        .allowlist_type("rte_flow_item_type_RTE_FLOW_ITEM_TYPE_IPV4")
        .allowlist_type("rte_flow_item_type_RTE_FLOW_ITEM_TYPE_TCP")
        .allowlist_type("rte_flow_item_type_RTE_FLOW_ITEM_TYPE_END")
        .allowlist_type("rte_flow_item_type_RTE_FLOW_ACTION_TYPE_END")
        .allowlist_type("rte_flow_item_type_RTE_FLOW_ACTION_TYPE_QUEUE")
        .allowlist_var("RTE_ETHER_MAX_JUMBO_FRAME")
        .allowlist_var("RTE_PKTMBUF_HEADROOM")
        .allowlist_var("RTE_ETH_LINK_UP")
        .allowlist_var("RTE_ETH_LINK_FULL_DUPLEX")
        .allowlist_var("RTE_MBUF_DEFAULT_BUF_SIZE")
        .allowlist_var("RTE_ETHER_MAX_JUMBO_FRAME_LEN")
        .allowlist_var("RTE_ETH_RX_OFFLOAD_IPV4_CKSUM")
        .allowlist_var("RTE_ETH_RX_OFFLOAD_TCP_CKSUM")
        .allowlist_var("RTE_ETH_RX_OFFLOAD_UDP_CKSUM")
        .allowlist_var("RTE_ETH_TX_OFFLOAD_IPV4_CKSUM")
        .allowlist_var("RTE_ETH_TX_OFFLOAD_TCP_CKSUM")
        .allowlist_var("RTE_ETH_TX_OFFLOAD_UDP_CKSUM")
        .allowlist_var("RTE_ETH_TX_OFFLOAD_MULTI_SEGS")
        .allowlist_var("RTE_ETH_DEV_NO_OWNER")
        .allowlist_var("RTE_ETHER_MAX_LEN")
        .allowlist_var("RTE_ETH_RSS_IP")
        .allowlist_var("RTE_ETH_RSS_TCP")
        .allowlist_var("RTE_ETH_RSS_UDP")
        .allowlist_var("RTE_MAX_ETHPORTS")
        .allowlist_var("RTE_ETH_MQ_RX_RSS")
        .allowlist_var("RTE_ETH_MQ_TX_NONE")
        .allowlist_function("rte_eth_find_next_owned_by")
        .allowlist_function("rte_eth_dev_info_get")
        .allowlist_function("rte_eth_macaddr_get")
        .allowlist_function("rte_auxiliary_register")
        .allowlist_function("rte_mempool_create_empty")
        .allowlist_function("rte_pktmbuf_pool_init")
        .allowlist_function("rte_mempool_populate_default")
        .allowlist_function("rte_pktmbuf_init")
        .allowlist_function("rte_mempool_avail_count")
        .allowlist_function("rte_mempool_in_use_count")
        .allowlist_function("rte_pktmbuf_clone")
        .allowlist_function("rte_eth_dev_start")
        .allowlist_function("rte_eth_dev_flow_ctrl_get")
        .allowlist_function("rte_strerror")
        .allowlist_function("rte_eth_dev_count_avail")
        .allowlist_function("rte_eth_conf")
        .allowlist_function("rte_eth_dev_configure")
        .allowlist_function("rte_eth_dev_count_avail")
        .allowlist_function("rte_eth_dev_get_mtu")
        .allowlist_function("rte_eth_dev_set_mtu")
        .allowlist_function("rte_eth_promiscuous_enable")
        .allowlist_function("rte_eth_dev_is_valid_port")
        .allowlist_function("rte_eth_dev_flow_ctrl_set")
        .allowlist_function("rte_mempool_avail_count")
        .allowlist_function("rte_mempool_in_use_count")
        .allowlist_function("rte_eth_link_get_nowait")
        .allowlist_function("rte_delay_us_block")
        .allowlist_function("rte_socket_id")
        .allowlist_function("rte_pktmbuf_pool_create")
        .allowlist_function("rte_eth_dev_socket_id")
        .allowlist_function("rte_eth_dev_socket_id")
        .allowlist_function("rte_eth_rx_queue_setup")
        .allowlist_function("rte_eth_tx_queue_setup")
        .allowlist_function("rte_mempool_obj_iter")
        .allowlist_function("rte_mempool_mem_iter")
        .allowlist_function("rte_mempool_free")
        .allowlist_function("rte_eth_tx_burst")
        .allowlist_function("rte_eth_rx_burst")
        .allowlist_function("rte_eal_init")
        .allowlist_function("rte_eal_mp_wait_lcore")
        .allowlist_function("rte_eal_remote_launch")
        .allowlist_function("rte_flow_create")
        .allowlist_function("rte_flow_validate")
        .allowlist_function("rte_lcore_id")
        .allowlist_function("rte_lcore_count")
        .allowlist_function("rte_get_next_lcore")
        .allowlist_function("rte_get_timer_hz")
        .allowlist_function("rte_memcpy")
        .allowlist_function("rte_zmalloc")
        .allowlist_function("rte_pktmbuf_prepend")
        .clang_arg("-mavx")
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate_comments(false)
        .generate()
        .unwrap_or_else(|e| panic!("Failed to generate bindings: {:?}", e));
    let bindings_out = out_dir.join("bindings.rs");
    bindings.write_to_file(bindings_out).expect("Failed to write bindings");

    // Step 3: Compile a stub file so Rust can access `inline` functions in the headers
    // that aren't compiled into the libraries.
    let mut builder: Build = cc::Build::new();
    builder.opt_level(3);
    builder.pic(true);
    builder.flag("-march=native");
    builder.file("inlined.c");
    for header_location in &header_locations {
        builder.include(header_location);
    }
    builder.compile("inlined");
    Ok(())
}

fn main() {
    match os_build() {
        Ok(()) => {},
        Err(e) => panic!("Failed to generate bindings: {:?}", e),
    }
}
