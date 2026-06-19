-- ================================================================================ --
-- NEORV32 Minimal Wrapper - exposes XBUS for external memory simulation           --
-- -------------------------------------------------------------------------------- --
-- Minimal configuration: no internal IMEM/DMEM, no peripherals except CLINT.      --
-- All instruction/data accesses go out via the Wishbone-compatible XBUS.           --
-- -------------------------------------------------------------------------------- --
-- The NEORV32 RISC-V Processor - https://github.com/stnolting/neorv32              --
-- Copyright (c) NEORV32 contributors.                                              --
-- Copyright (c) 2020 - 2026 Stephan Nolting. All rights reserved.                  --
-- Licensed under the BSD-3-Clause license, see LICENSE for details.                --
-- SPDX-License-Identifier: BSD-3-Clause                                            --
-- ================================================================================ --

library ieee;
use ieee.std_logic_1164.all;

library neorv32;
use neorv32.neorv32_package.all;

entity neorv32_minimal_wrapper is
  port (
    -- Global control --
    clk_i      : in  std_ulogic;
    rstn_i     : in  std_ulogic;
    -- External bus interface (XBUS / Wishbone B4) --
    xbus_adr_o : out std_ulogic_vector(31 downto 0);
    xbus_dat_o : out std_ulogic_vector(31 downto 0);
    xbus_we_o  : out std_ulogic;
    xbus_sel_o : out std_ulogic_vector(3 downto 0);
    xbus_stb_o : out std_ulogic;
    xbus_cyc_o : out std_ulogic;
    xbus_dat_i : in  std_ulogic_vector(31 downto 0);
    xbus_ack_i : in  std_ulogic;
    -- Trap detection --
    trap_o     : out std_ulogic  -- pulses high when CPU enters any trap handler
  );
end entity;

architecture neorv32_minimal_wrapper_rtl of neorv32_minimal_wrapper is

  signal trace : trace_port_t;

begin

  trap_o <= trace.intr; -- high on first instruction of any trap handler

  neorv32_top_inst: neorv32_top
  generic map (
    -- Processor Clocking --
    CLOCK_FREQUENCY  => 100_000_000,
    -- Boot Configuration --
    BOOT_MODE_SELECT => 1,             -- boot from custom address (no bootloader, no IMEM image)
    BOOT_ADDR_CUSTOM => x"00000000",   -- fetch first instruction from address 0 (XBUS)
    -- RISC-V CPU Extensions --
    RISCV_ISA_C      => true,
    RISCV_ISA_M      => true,
    RISCV_ISA_Zicntr => true,
    -- Internal memories (disabled - all traffic goes to XBUS) --
    IMEM_EN          => false,
    DMEM_EN          => false,
    -- External bus interface --
    XBUS_EN          => true,
    XBUS_TIMEOUT     => 0,             -- no timeout
    XBUS_REGSTAGE_EN => false,
    -- Peripherals --
    IO_CLINT_EN      => true           -- core-local interruptor (timer)
  )
  port map (
    clk_i      => clk_i,
    rstn_i     => rstn_i,
    xbus_adr_o => xbus_adr_o,
    xbus_dat_o => xbus_dat_o,
    xbus_we_o  => xbus_we_o,
    xbus_sel_o => xbus_sel_o,
    xbus_stb_o => xbus_stb_o,
    xbus_cyc_o => xbus_cyc_o,
    xbus_dat_i     => xbus_dat_i,
    xbus_ack_i     => xbus_ack_i,
    xbus_err_i     => '0',
    trace_cpu0_o   => trace
  );

end architecture;
