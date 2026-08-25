/*****************************************************************************\
|                      Copyright (C) 2021-2025 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/





`default_nettype none

module hazard3_core #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
,
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// These really ought to be localparams, but are occasionally needed for
// passing flags around between modules, so are made available as parameters
// instead. It's ugly, but better scope hygiene than the preprocessor. These
// parameters should not be changed from their default values.

parameter W_REGADDR = 5,

parameter W_ALUOP   = 6,
parameter W_ALUSRC  = 1,
parameter W_MEMOP   = 5,
parameter W_BCOND   = 2,
parameter W_SHAMT   = 5,

parameter W_EXCEPT  = 4,
parameter W_MULOP   = 3
) (
	// Global signals
	input wire                 clk,
	input wire                 clk_always_on,
	input wire                 rst_n,

	// Power control signals
	output wire                pwrup_req,
	input  wire                pwrup_ack,
	output wire                clk_en,
	output reg                 unblock_out,
	input  wire                unblock_in,

	



	// Instruction fetch port
	output wire                bus_aph_req_i,
	output wire                bus_aph_panic_i, // e.g. branch mispredict + flush
	input  wire                bus_aph_ready_i,
	input  wire                bus_dph_ready_i,
	input  wire                bus_dph_err_i,

	output wire [W_ADDR-1:0]   bus_haddr_i,
	output wire [2:0]          bus_hsize_i,
	output wire                bus_priv_i,
	input  wire [W_DATA-1:0]   bus_rdata_i,

	// Load/store port
	output reg                 bus_aph_req_d,
	output wire                bus_aph_excl_d,
	input  wire                bus_aph_ready_d,
	input  wire                bus_dph_ready_d,
	input  wire                bus_dph_err_d,
	input  wire                bus_dph_exokay_d,

	output reg  [W_ADDR-1:0]   bus_haddr_d,
	output reg  [2:0]          bus_hsize_d,
	output reg                 bus_priv_d,
	output reg                 bus_hwrite_d,
	output reg  [W_DATA-1:0]   bus_wdata_d,
	input  wire [W_DATA-1:0]   bus_rdata_d,

	// Memory ordering signals
	output wire                fence_i_vld,
	output wire                fence_d_vld,
	input  wire                fence_rdy,

	// Debugger run/halt control
	input  wire                dbg_req_halt,
	input  wire                dbg_req_halt_on_reset,
	input  wire                dbg_req_resume,
	output wire                dbg_halted,
	output wire                dbg_running,
	// Debugger access to data0 CSR
	input  wire [W_DATA-1:0]   dbg_data0_rdata,
	output wire [W_DATA-1:0]   dbg_data0_wdata,
	output wire                dbg_data0_wen,
	// Debugger instruction injection
	input  wire [W_DATA-1:0]   dbg_instr_data,
	input  wire                dbg_instr_data_vld,
	output wire                dbg_instr_data_rdy,
	output wire                dbg_instr_caught_exception,
	output wire                dbg_instr_caught_ebreak,

	// Identification CSR values
	input  wire [W_DATA-1:0]   mhartid_val,
	input  wire [3:0]          eco_version,

	// Level-sensitive interrupt sources
	input  wire [NUM_IRQS-1:0] irq,       // -> mip.meip
	input  wire                soft_irq,  // -> mip.msip
	input  wire                timer_irq  // -> mip.mtip
);

/*****************************************************************************\
|                        Copyright (C) 2021 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// ALU operation selectors

localparam ALUOP_ADD     = 6'h00; 
localparam ALUOP_SUB     = 6'h01; 
localparam ALUOP_LT      = 6'h02;
localparam ALUOP_LTU     = 6'h04;
localparam ALUOP_AND     = 6'h06;
localparam ALUOP_OR      = 6'h07;
localparam ALUOP_XOR     = 6'h08;
localparam ALUOP_SRL     = 6'h09;
localparam ALUOP_SRA     = 6'h0a;
localparam ALUOP_SLL     = 6'h0b;
localparam ALUOP_MULDIV  = 6'h0c;
localparam ALUOP_RS2     = 6'h0d; // differs from AND/OR/XOR in [1:0]
// Bitmanip ALU operations (some also used by AMOs):
localparam ALUOP_SHXADD  = 6'h20;
localparam ALUOP_CLZ     = 6'h23;
localparam ALUOP_CPOP    = 6'h24;
localparam ALUOP_CTZ     = 6'h25;
localparam ALUOP_ANDN    = 6'h26; // Same LSBs as non-inverted
localparam ALUOP_ORN     = 6'h27; // Same LSBs as non-inverted
localparam ALUOP_XNOR    = 6'h28; // Same LSBs as non-inverted
localparam ALUOP_MAX     = 6'h29;
localparam ALUOP_MAXU    = 6'h2a;
localparam ALUOP_MIN     = 6'h2b;
localparam ALUOP_MINU    = 6'h2c;
localparam ALUOP_ORC_B   = 6'h2d;
localparam ALUOP_REV8    = 6'h2e;
localparam ALUOP_ROL     = 6'h2f;
localparam ALUOP_ROR     = 6'h30;
localparam ALUOP_SEXT_B  = 6'h31;
localparam ALUOP_SEXT_H  = 6'h32;
localparam ALUOP_ZEXT_H  = 6'h33;

localparam ALUOP_CLMUL   = 6'h34;

localparam ALUOP_BCLR    = 6'h35;
localparam ALUOP_BEXT    = 6'h36;
localparam ALUOP_BINV    = 6'h37;
localparam ALUOP_BSET    = 6'h38;

localparam ALUOP_PACK    = 6'h39;
localparam ALUOP_PACKH   = 6'h3a;
localparam ALUOP_BREV8   = 6'h3b;
localparam ALUOP_ZIP     = 6'h3c;
localparam ALUOP_UNZIP   = 6'h3d;

localparam ALUOP_BEXTM   = 6'h3e;

localparam ALUOP_XPERM   = 6'h3f;

// Parameters to control ALU input muxes. Bypass mux paths are
// controlled by X, so D has no parameters to choose these.

localparam ALUSRCA_RS1 = 1'h0;
localparam ALUSRCA_PC  = 1'h1;

localparam ALUSRCB_RS2 = 1'h0;
localparam ALUSRCB_IMM = 1'h1;

localparam MEMOP_LW   = 5'h00;
localparam MEMOP_LH   = 5'h01;
localparam MEMOP_LB   = 5'h02;
localparam MEMOP_LHU  = 5'h03;
localparam MEMOP_LBU  = 5'h04;
localparam MEMOP_SW   = 5'h05;
localparam MEMOP_SH   = 5'h06;
localparam MEMOP_SB   = 5'h07;

localparam MEMOP_LR_W = 5'h08;
localparam MEMOP_SC_W = 5'h09;
localparam MEMOP_AMO  = 5'h0a;
localparam MEMOP_NONE = 5'h10;

localparam BCOND_NEVER  = 2'h0;
localparam BCOND_ALWAYS = 2'h1;
localparam BCOND_ZERO   = 2'h2;
localparam BCOND_NZERO  = 2'h3;

// CSR access types

localparam CSR_WTYPE_W    = 2'h0;
localparam CSR_WTYPE_S    = 2'h1;
localparam CSR_WTYPE_C    = 2'h2;

// Exceptional condition signals which travel alongside (or instead of)
// instructions in the pipeline. These are speculative and can be flushed on
// e.g. branch mispredict. These mostly align with mcause values.

localparam EXCEPT_NONE           = 4'hf;

localparam EXCEPT_INSTR_MISALIGN = 4'h0;
localparam EXCEPT_INSTR_FAULT    = 4'h1;
localparam EXCEPT_INSTR_ILLEGAL  = 4'h2;
localparam EXCEPT_EBREAK         = 4'h3;
localparam EXCEPT_LOAD_ALIGN     = 4'h4;
localparam EXCEPT_LOAD_FAULT     = 4'h5;
localparam EXCEPT_STORE_ALIGN    = 4'h6;
localparam EXCEPT_STORE_FAULT    = 4'h7;
localparam EXCEPT_ECALL_U        = 4'h8;
// MRET, Return from M-mode: not really an exception, but handled like one
localparam EXCEPT_MRET           = 4'ha;
localparam EXCEPT_ECALL_M        = 4'hb;
// spare: c
// spare: d
// REFETCH: flush and refetch sequentially-following instructions, e.g. on
// executing fence.i. Jumps from stage 3 to get ordering against L/S dphase.
localparam EXCEPT_REFETCH        = 4'he;

// Operations for M extension (these are just instr[14:12])

localparam M_OP_MUL    = 3'h0;
localparam M_OP_MULH   = 3'h1;
localparam M_OP_MULHSU = 3'h2;
localparam M_OP_MULHU  = 3'h3;
localparam M_OP_DIV    = 3'h4;
localparam M_OP_DIVU   = 3'h5;
localparam M_OP_REM    = 3'h6;
localparam M_OP_REMU   = 3'h7;
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

localparam RV_RS1_LSB = 15;
localparam RV_RS1_BITS = 5;
localparam RV_RS2_LSB = 20;
localparam RV_RS2_BITS = 5;
localparam RV_RD_LSB = 7;
localparam RV_RD_BITS = 5;

// Note: these are preprocessor macros, rather than the usual localparams,
// because it's quite difficult to get a definitive citation from 1364-2005
// for whether Z values are propagated through a localparam to a casez.
// Multiple tools complain about it, so just this once I'll use macros.




// Base ISA (some of these are Z now)



















































// M extension









// A extension












// Zba (address generation)




// Zbb (basic bit manipulation)



















// Zbc (carry-less multiply)




// Zbs (single-bit manipulation)









// Zbkb (basic bit manipulation for crypto) (minus those in Zbb)






// Zbkc is a subset of Zbc.

// Zbkx (crossbar permutation)



// Zilsd (load/store pair)



// Hazard3 custom instructions

// Xh3bextm (Hazard3 multi-bit extract): multi-bit versions of bext/bexti from Zbs



// C Extension








// addi16sp when rd=2:












// jr if !rs2:

// jalr if !rs2:




// Zcb simple additional compressed instructions












// Zclsd load/store pair instructions





// Zcmp push/pop instructions







// Copies provided here with 0 instead of ? so that these can be used to build 32-bit instructions in the decompressor


















































// Non-RV32I instructions for Zcb:





// Non-RV32I instructions for Zclsd:





wire x_stall;
wire m_stall;

localparam HSIZE_WORD  = 3'd2;
localparam HSIZE_HWORD = 3'd1;
localparam HSIZE_BYTE  = 3'd0;

localparam [4:0] REGADDR_MASK = {~|EXTENSION_E, 4'hf};

wire debug_mode;
assign dbg_halted = DEBUG_SUPPORT && debug_mode;
assign dbg_running = DEBUG_SUPPORT && !debug_mode;

// ----------------------------------------------------------------------------
// Pipe Stage F

wire                 f_jump_req;
wire [W_ADDR-1:0]    f_jump_target;
wire                 f_jump_priv;
wire                 f_jump_rdy;
wire                 f_jump_now = f_jump_req && f_jump_rdy;

wire                 f_frontend_pwrdown_ok;

// Predecoded register numbers, for register file access
wire [W_REGADDR-1:0] f_rs1_coarse;
wire [W_REGADDR-1:0] f_rs2_coarse;
wire [W_REGADDR-1:0] f_rs1_fine;
wire [W_REGADDR-1:0] f_rs2_fine;

wire [31:0]          fd_cir;
wire [31:0]          fd_cir_raw;
wire [1:0]           fd_cir_err;
wire                 fd_cir_break_any;
wire                 fd_cir_break_d_mode;
wire [1:0]           fd_cir_predbranch;
wire [1:0]           fd_cir_vld;
wire                 fd_cir_is_32bit;
wire                 fd_cir_invalid_16bit;
wire                 fd_cir_is_uop;
wire                 fd_cir_uop_nonfinal;
wire                 fd_cir_uop_no_pc_update;
wire                 fd_cir_uop_atomic;

wire [1:0]           df_cir_use;
wire                 df_cir_flush_behind;

wire                 df_uop_stall;
wire                 df_uop_clear;
wire                 df_lspair_phase_next;

wire                 x_btb_set;
wire [W_ADDR-1:0]    x_btb_set_src_addr;
wire                 x_btb_set_src_size;
wire [W_ADDR-1:0]    x_btb_set_target_addr;
wire                 x_btb_clear;

wire [W_ADDR-1:0]    d_btb_target_addr;

wire [W_ADDR-1:0]    f_pmp_i_addr;
wire                 f_pmp_i_m_mode;
wire                 f_pmp_i_kill;

wire [W_ADDR-1:0]    f_trigger_addr;
wire                 f_trigger_m_mode;
wire [1:0]           f_trigger_break_any;
wire [1:0]           f_trigger_break_d_mode;

assign bus_aph_panic_i = 1'b0;

wire f_mem_size;
assign bus_hsize_i = f_mem_size ? HSIZE_WORD : HSIZE_HWORD;

hazard3_frontend #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
) frontend (
	.clk                  (clk),
	.rst_n                (rst_n),

	.mem_size             (f_mem_size),
	.mem_addr             (bus_haddr_i),
	.mem_priv             (bus_priv_i),
	.mem_addr_vld         (bus_aph_req_i),
	.mem_addr_rdy         (bus_aph_ready_i),

	.mem_data             (bus_rdata_i),
	.mem_data_err         (bus_dph_err_i),
	.mem_data_vld         (bus_dph_ready_i),

	.jump_target          (f_jump_target),
	.jump_priv            (f_jump_priv),
	.jump_target_vld      (f_jump_req),
	.jump_target_rdy      (f_jump_rdy),

	.btb_set              (x_btb_set),
	.btb_set_src_addr     (x_btb_set_src_addr),
	.btb_set_src_size     (x_btb_set_src_size),
	.btb_set_target_addr  (x_btb_set_target_addr),
	.btb_clear            (x_btb_clear),
	.btb_target_addr_out  (d_btb_target_addr),

	.cir                  (fd_cir),
	.cir_raw              (fd_cir_raw),
	.cir_err              (fd_cir_err),
	.cir_predbranch       (fd_cir_predbranch),
	.cir_break_any        (fd_cir_break_any),
	.cir_break_d_mode     (fd_cir_break_d_mode),
	.cir_vld              (fd_cir_vld),
	.cir_is_32bit         (fd_cir_is_32bit),
	.cir_invalid_16bit    (fd_cir_invalid_16bit),
	.cir_is_uop           (fd_cir_is_uop),
	.cir_uop_nonfinal     (fd_cir_uop_nonfinal),
	.cir_uop_no_pc_update (fd_cir_uop_no_pc_update),
	.cir_uop_atomic       (fd_cir_uop_atomic),

	.cir_use              (df_cir_use),
	.cir_flush_behind     (df_cir_flush_behind),
	.uop_stall            (df_uop_stall),
	.uop_clear            (df_uop_clear),
	.df_lspair_phase_next (df_lspair_phase_next),

	.pwrdown_ok           (f_frontend_pwrdown_ok),
	.delay_first_fetch    (!pwrup_ack),

	.predecode_rs1_coarse (f_rs1_coarse),
	.predecode_rs2_coarse (f_rs2_coarse),
	.predecode_rs1_fine   (f_rs1_fine),
	.predecode_rs2_fine   (f_rs2_fine),

	.debug_mode           (debug_mode),
	.dbg_instr_data       (dbg_instr_data),
	.dbg_instr_data_vld   (dbg_instr_data_vld),
	.dbg_instr_data_rdy   (dbg_instr_data_rdy),

	.pmp_i_addr           (f_pmp_i_addr),
	.pmp_i_m_mode         (f_pmp_i_m_mode),
	.pmp_i_kill           (f_pmp_i_kill),

	.trigger_addr         (f_trigger_addr),
	.trigger_m_mode       (f_trigger_m_mode),
	.trigger_break_any    (f_trigger_break_any),
	.trigger_break_d_mode (f_trigger_break_d_mode)
);

// ----------------------------------------------------------------------------
// Pipe Stage X (Decode Logic)

// X-check on pieces of instruction which frontend claims are valid















// To X
wire                 d_starved;
wire [W_DATA-1:0]    d_imm;
wire [W_REGADDR-1:0] d_rs1;
wire [W_REGADDR-1:0] d_rs2;
wire [W_REGADDR-1:0] d_rd;
wire [2:0]           d_funct3_32b;
wire [6:0]           d_funct7_32b;
wire [W_ALUSRC-1:0]  d_alusrc_a;
wire [W_ALUSRC-1:0]  d_alusrc_b;
wire [W_ALUOP-1:0]   d_aluop;
wire [W_MEMOP-1:0]   d_memop;
wire [W_MULOP-1:0]   d_mulop;
wire [W_BCOND-1:0]   d_branchcond;
wire [W_ADDR-1:0]    d_addr_offs;
wire                 d_addr_is_regoffs;
wire [W_ADDR-1:0]    d_pc;
wire [W_EXCEPT-1:0]  d_except;
wire                 d_sleep_wfi;
wire                 d_sleep_block;
wire                 d_sleep_unblock;
wire                 d_no_pc_increment;
wire                 d_uninterruptible;
wire                 d_fence_i;
wire                 d_fence_d;
wire                 d_csr_ren;
wire                 d_csr_wen;
wire [1:0]           d_csr_wtype;
wire                 d_csr_w_imm;
wire [W_ADDR-1:0]    d_lspair_offset;

wire                 x_jump_not_except;
wire                 x_mmode_execution;
wire                 x_trap_wfi;

wire [W_ADDR-1:0]    debug_dpc_wdata;
wire                 debug_dpc_wen;
wire [W_ADDR-1:0]    debug_dpc_rdata;

hazard3_decode #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
) decode_u (
	.clk                     (clk),
	.rst_n                   (rst_n),

	.fd_cir                  (fd_cir),
	.fd_cir_err              (fd_cir_err),
	.fd_cir_predbranch       (fd_cir_predbranch),
	.fd_cir_vld              (fd_cir_vld),
	.fd_cir_is_32bit         (fd_cir_is_32bit),
	.fd_cir_invalid_16bit    (fd_cir_invalid_16bit),
	.fd_cir_is_uop           (fd_cir_is_uop),
	.fd_cir_uop_nonfinal     (fd_cir_uop_nonfinal),
	.fd_cir_uop_no_pc_update (fd_cir_uop_no_pc_update),
	.fd_cir_uop_atomic       (fd_cir_uop_atomic),

	.df_cir_use              (df_cir_use),
	.df_cir_flush_behind     (df_cir_flush_behind),
	.df_uop_stall            (df_uop_stall),
	.df_uop_clear            (df_uop_clear),
	.df_lspair_phase_next    (df_lspair_phase_next),

	.d_pc                    (d_pc),
	.x_jump_not_except       (x_jump_not_except),

	.debug_mode              (debug_mode),
	.m_mode                  (x_mmode_execution),
	.trap_wfi                (x_trap_wfi),

	.debug_dpc_wdata         (debug_dpc_wdata),
	.debug_dpc_wen           (debug_dpc_wen),
	.debug_dpc_rdata         (debug_dpc_rdata),

	.d_starved               (d_starved),
	.x_stall                 (x_stall),
	.f_jump_now              (f_jump_now),
	.f_jump_target           (f_jump_target),
	.d_btb_target_addr       (d_btb_target_addr),

	.d_imm                   (d_imm),
	.d_rs1                   (d_rs1),
	.d_rs2                   (d_rs2),
	.d_rd                    (d_rd),
	.d_funct3_32b            (d_funct3_32b),
	.d_funct7_32b            (d_funct7_32b),
	.d_alusrc_a              (d_alusrc_a),
	.d_alusrc_b              (d_alusrc_b),
	.d_aluop                 (d_aluop),
	.d_memop                 (d_memop),
	.d_mulop                 (d_mulop),
	.d_csr_ren               (d_csr_ren),
	.d_csr_wen               (d_csr_wen),
	.d_csr_wtype             (d_csr_wtype),
	.d_csr_w_imm             (d_csr_w_imm),
	.d_branchcond            (d_branchcond),
	.d_addr_offs             (d_addr_offs),
	.d_addr_is_regoffs       (d_addr_is_regoffs),
	.d_except                (d_except),
	.d_sleep_wfi             (d_sleep_wfi),
	.d_sleep_block           (d_sleep_block),
	.d_sleep_unblock         (d_sleep_unblock),
	.d_no_pc_increment       (d_no_pc_increment),
	.d_uninterruptible       (d_uninterruptible),
	.d_lspair_offset         (d_lspair_offset),
	.d_fence_i               (d_fence_i),
	.d_fence_d               (d_fence_d)
);

// ----------------------------------------------------------------------------
// Pipe Stage X (Execution Logic)

// Register the write which took place to the regfile on previous cycle, and bypass.
// This is an alternative to a write -> read bypass in the regfile,
// which we can't implement whilst maintaining BRAM inference compatibility (iCE40).
reg  [W_REGADDR-1:0] mw_rd;
reg  [W_DATA-1:0]    mw_result;

// From register file:
wire [W_DATA-1:0]    x_rdata1;
wire [W_DATA-1:0]    x_rdata2;

// Combinational regs for muxing
reg   [W_DATA-1:0]   x_rs1_bypass;
reg   [W_DATA-1:0]   x_rs2_bypass;
reg   [W_DATA-1:0]   x_op_a;
reg   [W_DATA-1:0]   x_op_b;
wire  [W_DATA-1:0]   x_alu_result;
wire                 x_alu_cmp;

wire [W_DATA-1:0]    m_trap_addr;
wire                 m_trap_is_irq;
wire                 m_trap_enter_vld;
wire                 m_trap_enter_soon;
wire                 m_trap_enter_rdy = f_jump_rdy;

wire                 x_mmode_loadstore;
wire                 m_mmode_trap_entry;

reg  [W_REGADDR-1:0] xm_rs1;
reg  [W_REGADDR-1:0] xm_rs2;
reg  [W_REGADDR-1:0] xm_rd;
reg  [W_DATA-1:0]    xm_result;
reg  [1:0]           xm_addr_align;
reg  [W_MEMOP-1:0]   xm_memop;
reg  [W_EXCEPT-1:0]  xm_except;
reg                  xm_except_to_d_mode;
reg                  xm_no_pc_increment;
reg                  xm_sleep_wfi;
reg                  xm_sleep_block;
reg                  xm_delay_irq_entry_on_ls_stagex;

// ----------------------------------------------------------------------------
// Stall logic

// IRQs squeeze in between the instructions in X and M, so in this case X
// stalls but M can continue. -> X always stalls on M trap, M *may* stall.
wire x_stall_on_trap = m_trap_enter_vld && !m_trap_enter_rdy ||
	m_trap_enter_soon && !m_trap_enter_vld;

// Stall inserted to avoid illegal pipelining of exclusive accesses on the bus
// (also gives time to update local monitor on direct lr.w -> sc.w instruction
// sequences). Note we don't check for AMOs in stage M, because AMOs fully
// fence off on their own completion before passing down the pipe.

wire d_memop_is_amo = |EXTENSION_A && d_memop == MEMOP_AMO;

wire x_stall_on_exclusive_overlap = |EXTENSION_A && (
	(d_memop_is_amo || d_memop == MEMOP_SC_W || d_memop == MEMOP_LR_W) &&
	(xm_memop == MEMOP_SC_W || xm_memop	== MEMOP_LR_W)
);

// AMOs are issued completely from X. We keep X stalled, and pass bubbles into
// M. Otherwise the exception handling would be even more of a mess. Phases
// 0-3 are read/write address/data phases. Phase 4 is error, due to HRESP or
// due to low HEXOKAY response to read.

// Also need to clear AMO if it follows an excepting instruction. Note we
// still stall on phase 3 when hready is high if hresp is also high, since we
// then proceed to phase 4 for the error response.

reg [2:0] x_amo_phase;
wire x_stall_on_amo = |EXTENSION_A && d_memop_is_amo && !m_trap_enter_soon && (
	x_amo_phase < 3'h3 || (x_amo_phase == 3'h3 && (!bus_dph_ready_d || !bus_dph_exokay_d || bus_dph_err_d))
);

// Read-after-write hazard detection (e.g. load-use)

wire m_fast_mul_result_vld;
wire m_generating_result =
	(xm_memop < MEMOP_SW) ||
	(|EXTENSION_A && xm_memop == MEMOP_LR_W) ||
	(|EXTENSION_A && xm_memop == MEMOP_SC_W) || // sc.w success result is data phase
	(|EXTENSION_M && m_fast_mul_result_vld);

reg x_stall_on_raw;

always @ (*) begin
	x_stall_on_raw = 1'b0;
	if (REDUCED_BYPASS) begin
		x_stall_on_raw =
			|xm_rd && (xm_rd == d_rs1 || xm_rd == d_rs2) ||
			|mw_rd && (mw_rd == d_rs1 || mw_rd == d_rs2);
	end else if (m_generating_result) begin
		// With the full bypass network, load-use (or fast multiply-use) is the only RAW stall
		if (|xm_rd && xm_rd == d_rs1) begin
			// Store addresses cannot be bypassed later, so there is no exception here.
			x_stall_on_raw = 1'b1;
		end else if (|xm_rd && xm_rd == d_rs2) begin
			// Store data can be bypassed in M. Any other instructions must stall.
			x_stall_on_raw = !(d_memop == MEMOP_SW || d_memop == MEMOP_SH || d_memop == MEMOP_SB);
		end
	end
end

wire x_stall_on_fence;
wire x_stall_muldiv;
wire x_jump_req;

assign x_stall =
	m_stall ||
	x_stall_on_trap ||
	x_stall_on_exclusive_overlap ||
	x_stall_on_amo ||
	x_stall_on_raw ||
	x_stall_on_fence ||
	x_stall_muldiv ||
	bus_aph_req_d && !bus_aph_ready_d ||
	x_jump_req && !f_jump_rdy;

wire m_sleep_stall_release;

wire x_loadstore_pmp_fail;
wire x_trig_step_break;
wire x_trig_break = fd_cir_break_any || x_trig_step_break;
wire x_trig_break_d_mode = fd_cir_break_d_mode;

// ----------------------------------------------------------------------------
// Execution logic

// ALU, operand muxes and bypass

// Approximate regnums were predecoded in stage 1, for regfile read.
// (Approximate in the sense that they are invalid when the instruction
// doesn't *have* a register operand on that port.) These aren't usable for
// hazard checking but are fine for bypass, and make the bypass mux
// independent of stage 2 decode.

reg [W_REGADDR-1:0] d_rs1_predecoded;
reg [W_REGADDR-1:0] d_rs2_predecoded;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		d_rs1_predecoded <= {W_REGADDR{1'b0}};
		d_rs2_predecoded <= {W_REGADDR{1'b0}};
	end else if (d_starved || !x_stall) begin
		d_rs1_predecoded <= f_rs1_fine & REGADDR_MASK;
		d_rs2_predecoded <= f_rs2_fine & REGADDR_MASK;
	end
end

wire [W_REGADDR-1:0] d_rs1_predecoded_nxt = REGADDR_MASK & (
	d_starved || !x_stall ? f_rs1_fine : d_rs1_predecoded
);
wire [W_REGADDR-1:0] d_rs2_predecoded_nxt = REGADDR_MASK & (
	d_starved || !x_stall ? f_rs2_fine : d_rs2_predecoded
);

wire [W_REGADDR-1:0] xm_rd_nxt;
wire [W_REGADDR-1:0] mw_rd_nxt;

reg [2:0] x_rs1_select_regs_xm_mw;
reg [2:0] x_rs2_select_regs_xm_mw;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		x_rs1_select_regs_xm_mw <= 3'b000;
		x_rs2_select_regs_xm_mw <= 3'b000;
	end else begin
		x_rs1_select_regs_xm_mw <=
			5'h00     == d_rs1_predecoded_nxt ? 3'b000 :
			xm_rd_nxt == d_rs1_predecoded_nxt ? 3'b010 :
			mw_rd_nxt == d_rs1_predecoded_nxt ? 3'b001 : 3'b100;
		x_rs2_select_regs_xm_mw <=
			5'h00     == d_rs2_predecoded_nxt ? 3'b000 :
			xm_rd_nxt == d_rs2_predecoded_nxt ? 3'b010 :
			mw_rd_nxt == d_rs2_predecoded_nxt ? 3'b001 : 3'b100;
	end
end


























always @ (*) begin

	x_rs1_bypass =
		(x_rdata1  & {W_DATA{x_rs1_select_regs_xm_mw[2]}}) |
		(xm_result & {W_DATA{x_rs1_select_regs_xm_mw[1]}}) |
		(mw_result & {W_DATA{x_rs1_select_regs_xm_mw[0]}});

	x_rs2_bypass =
		(x_rdata2  & {W_DATA{x_rs2_select_regs_xm_mw[2]}}) |
		(xm_result & {W_DATA{x_rs2_select_regs_xm_mw[1]}}) |
		(mw_result & {W_DATA{x_rs2_select_regs_xm_mw[0]}});

	// AMO captures rdata into mw_result at end of read data phase, so we can
	// feed back through the ALU.
	if (|EXTENSION_A && x_amo_phase == 3'h2)
		x_op_a = mw_result;
	else if (d_alusrc_a)
		x_op_a = d_pc;
	else
		x_op_a = x_rs1_bypass;

	if (d_alusrc_b)
		x_op_b = d_imm;
	else
		x_op_b = x_rs2_bypass;
end

hazard3_alu #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
) alu (
	.aluop      (d_aluop),
	.funct3_32b (d_funct3_32b),
	.funct7_32b (d_funct7_32b),
	.op_a       (x_op_a),
	.op_b       (x_op_b),
	.result     (x_alu_result),
	.cmp        (x_alu_cmp)
);

// Load/store bus request

wire x_unaligned_addr = d_memop != MEMOP_NONE && (
	bus_hsize_d == HSIZE_WORD && |bus_haddr_d[1:0] ||
	bus_hsize_d == HSIZE_HWORD && bus_haddr_d[0]
);

reg mw_local_exclusive_reserved;

wire x_memop_vld = d_memop != MEMOP_NONE && !(
	|EXTENSION_A && d_memop == MEMOP_SC_W && !mw_local_exclusive_reserved ||
	|EXTENSION_A && d_memop_is_amo && x_amo_phase != 3'h0 && x_amo_phase != 3'h2
);

wire x_memop_write =
	d_memop == MEMOP_SW ||
	d_memop == MEMOP_SH ||
	d_memop == MEMOP_SB ||
	(|EXTENSION_A && d_memop == MEMOP_SC_W) ||
	(|EXTENSION_A && d_memop_is_amo && x_amo_phase == 3'h2);

// Always query the global monitor, except for store-conditional suppressed by local monitor.
assign bus_aph_excl_d = |EXTENSION_A && (
	d_memop == MEMOP_LR_W ||
	d_memop == MEMOP_SC_W ||
	d_memop_is_amo
);

// AMO stalls the pipe, then generates two bus transfers per 4-cycle
// iteration, unless it bails out due to a bus fault or failed load
// reservation.
always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		x_amo_phase <= 3'h0;
	end else if (|EXTENSION_A && d_memop_is_amo && (
		bus_aph_ready_d || bus_dph_ready_d ||
		m_trap_enter_vld || x_unaligned_addr || x_loadstore_pmp_fail ||
		x_amo_phase == 3'h4
	)) begin
		if (m_trap_enter_vld) begin
			// Bail out, squash the in-progress AMO.
			x_amo_phase <= 3'h0;
		end else if (x_stall_on_raw || x_stall_on_exclusive_overlap || m_trap_enter_soon) begin
			// First address phase stalled due to address dependency on
			// previous load/mul/etc. Shouldn't be possible in later phases.
			x_amo_phase <= 3'h0;
		end else if (x_amo_phase == 3'h4) begin
			// Clear fault phase once it goes through to stage 3 and excepts
			if (!x_stall)
				x_amo_phase <= 3'h0;
		end else if (x_unaligned_addr || x_loadstore_pmp_fail) begin
			x_amo_phase <= 3'h4;
		end else if (x_amo_phase == 3'h1 && !bus_dph_exokay_d) begin
			// Load reserve fail indicates the memory region does not support
			// exclusives, so we will never succeed at store. Exception.
			x_amo_phase <= 3'h4;
		end else if ((x_amo_phase == 3'h1 || x_amo_phase == 3'h3) && bus_dph_err_d) begin
			// Bus fault. Exception.
			x_amo_phase <= 3'h4;
		end else if (x_amo_phase == 3'h3) begin
			// Either we're done, or the write failed. Either way, back to the start.
			x_amo_phase <= 3'h0;
		end else begin
			// Default progression: read addr -> read data -> write addr -> write data
			x_amo_phase <= x_amo_phase + 3'h1;
		end
	end
end











































































// This adder is used for both branch targets and load/store addresses.
// Supporting all branch types already requires rs1 + I-fmt, and pc + B-fmt.
// B-fmt are almost identical to S-fmt, so we rs1 + S-fmt is almost free.

wire [W_ADDR-1:0] x_addr_sum =
	(d_lspair_offset & {W_ADDR{|EXTENSION_ZILSD}}) +
	(d_addr_is_regoffs ? x_rs1_bypass : d_pc) +
	d_addr_offs;

always @ (*) begin
	// Need to be careful not to use anything hready-sourced to gate htrans!
	bus_haddr_d = x_addr_sum;
	bus_hwrite_d = x_memop_write;
	bus_priv_d = x_mmode_loadstore;
	case (d_memop)
		MEMOP_LW:  bus_hsize_d = HSIZE_WORD;
		MEMOP_SW:  bus_hsize_d = HSIZE_WORD;
		MEMOP_LH:  bus_hsize_d = HSIZE_HWORD;
		MEMOP_LHU: bus_hsize_d = HSIZE_HWORD;
		MEMOP_SH:  bus_hsize_d = HSIZE_HWORD;
		MEMOP_LB:  bus_hsize_d = HSIZE_BYTE;
		MEMOP_LBU: bus_hsize_d = HSIZE_BYTE;
		MEMOP_SB:  bus_hsize_d = HSIZE_BYTE;
		default:   bus_hsize_d = HSIZE_WORD;
	endcase
	bus_aph_req_d = x_memop_vld && !(
		x_stall_on_raw ||
		x_stall_on_exclusive_overlap ||
		x_loadstore_pmp_fail ||
		x_trig_break ||
		x_unaligned_addr ||
		m_trap_enter_soon ||
		((xm_sleep_wfi || xm_sleep_block) && !m_sleep_stall_release)
	);
end

// Fences are not issued until relevant buses are quiet.
wire m_dphase_in_flight;
assign x_stall_on_fence =
	(d_fence_i && !(fence_rdy && !m_dphase_in_flight && f_frontend_pwrdown_ok)) ||
	(d_fence_d && !(fence_rdy && !m_dphase_in_flight));

wire x_fence_mask =
	x_trig_break ||
	m_dphase_in_flight ||
	m_trap_enter_soon ||
	((xm_sleep_wfi || xm_sleep_block) && !m_sleep_stall_release);

assign fence_i_vld = d_fence_i && !x_fence_mask && f_frontend_pwrdown_ok;
assign fence_d_vld = d_fence_d && !x_fence_mask;

// Multiply/divide

wire [W_DATA-1:0] x_muldiv_result;
wire [W_DATA-1:0] m_fast_mul_result;

wire x_use_fast_mul = d_aluop == ALUOP_MULDIV && (
	(MUL_FAST  && d_mulop == M_OP_MUL   ) ||
	(MULH_FAST && d_mulop == M_OP_MULH  ) ||
	(MULH_FAST && d_mulop == M_OP_MULHU ) ||
	(MULH_FAST && d_mulop == M_OP_MULHSU)
);

generate
if (EXTENSION_M) begin: has_muldiv
	wire              x_muldiv_op_vld;
	wire              x_muldiv_op_rdy;
	wire              x_muldiv_result_vld;
	wire [W_DATA-1:0] x_muldiv_result_h;
	wire [W_DATA-1:0] x_muldiv_result_l;

	reg x_muldiv_posted;
	always @ (posedge clk or negedge rst_n)
		if (!rst_n)
			x_muldiv_posted <= 1'b0;
		else
			x_muldiv_posted <= (x_muldiv_posted || (x_muldiv_op_vld && x_muldiv_op_rdy)) && x_stall;

	wire x_muldiv_kill = m_trap_enter_soon;

	assign x_muldiv_op_vld = (d_aluop == ALUOP_MULDIV && !x_use_fast_mul)
		&& !(x_muldiv_posted || x_stall_on_raw || x_muldiv_kill);

	hazard3_muldiv_seq #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
	) muldiv (
		.clk        (clk),
		.rst_n      (rst_n),
		.op         (d_mulop),
		.op_vld     (x_muldiv_op_vld),
		.op_rdy     (x_muldiv_op_rdy),
		.op_kill    (x_muldiv_kill),
		.op_a       (x_rs1_bypass),
		.op_b       (x_rs2_bypass),

		.result_h   (x_muldiv_result_h),
		.result_l   (x_muldiv_result_l),
		.result_vld (x_muldiv_result_vld)
	);

	wire x_muldiv_result_is_high =
		d_mulop == M_OP_MULH ||
		d_mulop == M_OP_MULHSU ||
		d_mulop == M_OP_MULHU ||
		d_mulop == M_OP_REM ||
		d_mulop == M_OP_REMU;
	assign x_muldiv_result = x_muldiv_result_is_high ? x_muldiv_result_h : x_muldiv_result_l;
	assign x_stall_muldiv = x_muldiv_op_vld || !x_muldiv_result_vld;

	if (MUL_FAST) begin: has_fast_mul

		// If MUL_FASTER is set, the multiplier produces its result
		// combinatorially, so we never have to post a mul result to stage 3.
		wire x_issue_fast_mul = x_use_fast_mul && |d_rd && !x_stall && !MUL_FASTER;

		hazard3_mul_fast #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
		) mul_fast (
			.clk        (clk),
			.rst_n      (rst_n),

			.op_vld     (x_issue_fast_mul),
			.op         (d_mulop),
			.op_a       (x_rs1_bypass),
			.op_b       (x_rs2_bypass),

			.result     (m_fast_mul_result),
			.result_vld (m_fast_mul_result_vld)
		);

	end else begin: no_fast_mul

		assign m_fast_mul_result = {W_DATA{1'b0}};
		assign m_fast_mul_result_vld = 1'b0;

	end





end else begin: no_muldiv

	assign x_muldiv_result = {W_DATA{1'b0}};
	assign m_fast_mul_result = {W_DATA{1'b0}};
	assign m_fast_mul_result_vld = 1'b0;
	assign x_stall_muldiv = 1'b0;

end
endgenerate

// Branch handling

// For JALR, the LSB of the result must be cleared by hardware
wire [W_ADDR-1:0] x_jump_target = x_addr_sum & ~32'd1;
wire              x_jump_misaligned = ~|EXTENSION_C && x_addr_sum[1];
wire              x_branch_cmp_noinvert;

wire              x_branch_was_predicted = |BRANCH_PREDICTOR && fd_cir_predbranch[0];
wire              x_branch_cmp = x_branch_cmp_noinvert ^ x_branch_was_predicted;

generate
if (~|FAST_BRANCHCMP) begin: alu_branchcmp

	assign x_branch_cmp_noinvert = x_alu_cmp;

end else begin: fast_branchcmp

	hazard3_branchcmp #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
	) branchcmp_u (
		.cir   (fd_cir),
		.op_a  (x_rs1_bypass),
		.op_b  (x_rs2_bypass),
		.cmp   (x_branch_cmp_noinvert)
	);

end
endgenerate

// Be careful not to take branches whose comparisons depend on a load result
wire x_jump_req_unchecked = !x_stall_on_raw && (
	d_branchcond == BCOND_ALWAYS ||
	d_branchcond == BCOND_ZERO && !x_branch_cmp ||
	d_branchcond == BCOND_NZERO && x_branch_cmp
);

assign x_jump_req = x_jump_req_unchecked && !x_jump_misaligned && !x_trig_break;

assign x_btb_set = |BRANCH_PREDICTOR && (
	x_jump_req_unchecked && d_addr_offs[W_ADDR - 1] && !x_branch_was_predicted &&
	(d_branchcond == BCOND_NZERO || d_branchcond == BCOND_ZERO)
);

assign x_btb_clear = d_fence_i || (m_trap_enter_vld && m_trap_enter_rdy) || (|BRANCH_PREDICTOR && (
	x_branch_was_predicted && x_jump_req_unchecked
));

assign x_btb_set_src_addr = d_pc;
assign x_btb_set_target_addr = x_jump_target;
assign x_btb_set_src_size = fd_cir_is_32bit;

// Memory protection

wire [11:0]       x_pmp_cfg_addr;
wire              x_pmp_cfg_wen;
wire [W_DATA-1:0] x_pmp_cfg_wdata;
wire [W_DATA-1:0] x_pmp_cfg_rdata;

generate
if (PMP_REGIONS > 0) begin: have_pmp

	wire x_loadstore_pmp_badperm;
	assign x_loadstore_pmp_fail = x_loadstore_pmp_badperm && x_memop_vld && !debug_mode;

	// R and W are enforced at the point the load/store address is generated,
	// in time to mask the address-phase request. X is enforced by checking
	// fetch addresses during fetch data phase (stage F) and promoting PMP
	// fails to bus faults.

	hazard3_pmp #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
	) pmp (
		.clk                       (clk),
		.rst_n                     (rst_n),

		.cfg_addr                  (x_pmp_cfg_addr),
		.cfg_wen                   (x_pmp_cfg_wen),
		.cfg_wdata                 (x_pmp_cfg_wdata),
		.cfg_rdata                 (x_pmp_cfg_rdata),

		.i_addr                    (f_pmp_i_addr),
		.i_m_mode                  (f_pmp_i_m_mode),
		.i_kill                    (f_pmp_i_kill),

		.d_addr                    (bus_haddr_d),
		.d_addr_addend_rs1         (x_rs1_bypass),
		.d_addr_addend_imm         (d_addr_offs),
		.d_addr_addend_lspair_offs (d_lspair_offset),
		.d_m_mode                  (x_mmode_loadstore),
		.d_write                   (bus_hwrite_d),
		.d_kill                    (x_loadstore_pmp_badperm)
	);

end else begin: no_pmp

	assign x_pmp_cfg_rdata = 32'd0;
	assign x_loadstore_pmp_fail = 1'b0;
	assign f_pmp_i_kill = 1'b0;

end
endgenerate

// Triggers

wire [11:0]       x_trig_cfg_addr;
wire              x_trig_cfg_wen;
wire [W_DATA-1:0] x_trig_cfg_wdata;
wire [W_DATA-1:0] x_trig_cfg_rdata;
wire              x_trig_m_en;

wire              m_trig_event_interrupt;
wire              m_trig_event_exception;
wire [3:0]        m_trig_event_trap_cause;

wire              x_instr_ret;

generate
if (DEBUG_SUPPORT != 0) begin: have_triggers

	// Connection of .fetch_d_mode is from the wrong stage: safe because we
	// enter/exit debug mode via exceptions, which flush the frontend.

	hazard3_triggers #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
	) triggers (
		.clk              (clk),
		.rst_n            (rst_n),

		.cfg_addr         (x_trig_cfg_addr),
		.cfg_wen          (x_trig_cfg_wen),
		.cfg_wdata        (x_trig_cfg_wdata),
		.cfg_rdata        (x_trig_cfg_rdata),
		.trig_m_en        (x_trig_m_en),

		.fetch_addr       (f_trigger_addr),
		.fetch_m_mode     (f_trigger_m_mode),
		.fetch_d_mode     (debug_mode),

		.event_instr_ret  (x_instr_ret),

		.event_interrupt  (m_trig_event_interrupt),
		.event_exception  (m_trig_event_exception),
		.event_trap_cause (m_trig_event_trap_cause),
		.event_trap_enter (m_trap_enter_vld && m_trap_enter_rdy),

		.break_any        (f_trigger_break_any),
		.break_d_mode     (f_trigger_break_d_mode),

		.break_m_step     (x_trig_step_break),

		.x_d_mode         (debug_mode),
		.x_m_mode         (x_mmode_execution)
	);

end else begin: no_triggers

	assign x_trig_cfg_rdata = {W_DATA{1'b0}};
	assign f_trigger_break_any = 2'b00;
	assign f_trigger_break_d_mode = 2'b00;
	assign x_trig_step_break = 1'b0;

end
endgenerate

// CSRs and Trap Handling

// Use opcode bits directly because d_rs1 is masked when RVE is enabled:
wire [W_DATA-1:0] x_csr_wdata = d_csr_w_imm ?
	{{W_DATA-5{1'b0}}, fd_cir[RV_RS1_LSB +: 5]} : x_rs1_bypass;

wire [W_DATA-1:0] x_csr_rdata;
wire              x_csr_illegal_access;
wire              x_csr_write_is_fetch_ordered;

// "Previous" refers to next-most-recent instruction to be in D/X, i.e. the
// most recent instruction to reach stage M (which may or may not still be in M).
reg prev_instr_was_32_bit;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		xm_delay_irq_entry_on_ls_stagex <= 1'b0;
		prev_instr_was_32_bit <= 1'b0;
	end else begin
		// Must hold off IRQ if we are in the second cycle of an address phase or
		// later, since at that point the load/store can't be revoked. The IRQ is
		// taken once this load/store moves to the next stage: if another load/store
		// is chasing down the pipeline then this is immediately suppressed by the
		// IRQ entry, before its address phase can begin.

		// Also hold off on AMOs, unless the AMO is transitioning to an address
		// phase or completing. ("completing" excludes transitions to error phase.)

		xm_delay_irq_entry_on_ls_stagex <= bus_aph_req_d && !bus_aph_ready_d ||
			d_memop_is_amo && !(
				x_amo_phase == 3'h3 && bus_dph_ready_d && !bus_dph_err_d ||
				// Read reservation failure failure also generates error
				x_amo_phase == 3'h1 && bus_dph_ready_d && !bus_dph_err_d && bus_dph_exokay_d
			) || (
				(fence_i_vld || fence_d_vld) && !fence_rdy
			);

		if (!x_stall)
			prev_instr_was_32_bit <= df_cir_use == 2'd2;
	end
end

wire [W_ADDR-1:0] m_exception_return_addr;

wire [W_EXCEPT-1:0] x_except =
	x_trig_break                                             ? EXCEPT_EBREAK         :
	x_jump_req_unchecked && x_jump_misaligned                ? EXCEPT_INSTR_MISALIGN :
	x_csr_illegal_access                                     ? EXCEPT_INSTR_ILLEGAL  :
	|EXTENSION_A && x_unaligned_addr &&  d_memop_is_amo      ? EXCEPT_STORE_ALIGN    :
	|EXTENSION_A && x_amo_phase == 3'h4 && x_unaligned_addr  ? EXCEPT_STORE_ALIGN    :
	|EXTENSION_A && x_amo_phase == 3'h4                      ? EXCEPT_STORE_FAULT    :
	x_unaligned_addr &&  x_memop_write                       ? EXCEPT_STORE_ALIGN    :
	x_unaligned_addr && !x_memop_write                       ? EXCEPT_LOAD_ALIGN     :
	x_loadstore_pmp_fail && (d_memop_is_amo || bus_hwrite_d) ? EXCEPT_STORE_FAULT    :
	x_loadstore_pmp_fail                                     ? EXCEPT_LOAD_FAULT     :
	x_csr_write_is_fetch_ordered                             ? EXCEPT_REFETCH        : d_except;

// If an instruction causes an exceptional condition we do not consider it to have retired.
wire x_except_counts_as_retire =
	x_except == EXCEPT_EBREAK  ||
	x_except == EXCEPT_REFETCH ||
	x_except == EXCEPT_MRET    ||
	x_except == EXCEPT_ECALL_M ||
	x_except == EXCEPT_ECALL_U;

assign x_instr_ret = |df_cir_use && (x_except == EXCEPT_NONE || x_except_counts_as_retire);

assign m_dphase_in_flight = xm_memop != MEMOP_NONE && xm_memop != MEMOP_AMO;

// Need to delay IRQ entry on sleep exit because, for deep sleep states, we
// can't access the bus until the power handshake has completed.
wire m_delay_irq_entry = xm_delay_irq_entry_on_ls_stagex ||
	((xm_sleep_wfi || xm_sleep_block) && !m_sleep_stall_release);

// Inhibit CSR read/write side effects; note most stage-X exception sources
// are excluded as these only apply to CSR instructions.
wire m_csr_access_mask_early = m_trap_enter_soon || fd_cir_break_any;
wire m_csr_access_mask = m_csr_access_mask_early || x_stall;

wire m_pwr_allow_clkgate;
wire m_pwr_allow_power_down;
wire m_pwr_allow_sleep_on_block;
wire m_wfi_wakeup_req;
wire m_clear_excl_on_trap_exit;

hazard3_csr #(
	.XLEN            (W_DATA),
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
) csr_u (
	.clk                        (clk),
	.clk_always_on              (clk_always_on),
	.rst_n                      (rst_n),

	// Debugger signalling
	.debug_mode                 (debug_mode),
	.dbg_req_halt               (dbg_req_halt),
	.dbg_req_halt_on_reset      (dbg_req_halt_on_reset),
	.dbg_req_resume             (dbg_req_resume),

	.dbg_instr_caught_exception (dbg_instr_caught_exception),
	.dbg_instr_caught_ebreak    (dbg_instr_caught_ebreak),

	.dbg_data0_rdata            (dbg_data0_rdata),
	.dbg_data0_wdata            (dbg_data0_wdata),
	.dbg_data0_wen              (dbg_data0_wen),

	.debug_dpc_wdata            (debug_dpc_wdata),
	.debug_dpc_wen              (debug_dpc_wen),
	.debug_dpc_rdata            (debug_dpc_rdata),

	// CSR access port
	// *en_soon are early access strobes which are not a function of bus stall.
	// Can generate access faults (hence traps), but do not actually perform access.
	.addr                       (fd_cir[31:20]), // Always I-type immediate
	.wdata                      (x_csr_wdata),
	.wen_raw                    (d_csr_wen),
	.wen_soon                   (d_csr_wen && !m_csr_access_mask_early),
	.wen                        (d_csr_wen && !m_csr_access_mask),
	.wtype                      (d_csr_wtype),
	.rdata                      (x_csr_rdata),
	.ren_soon                   (d_csr_ren && !m_csr_access_mask_early),
	.ren                        (d_csr_ren && !m_csr_access_mask),
	.illegal                    (x_csr_illegal_access),
	.write_is_fetch_ordered     (x_csr_write_is_fetch_ordered),

	// Trap signalling
	.trap_addr                  (m_trap_addr),
	.trap_is_irq                (m_trap_is_irq),
	.trap_enter_soon            (m_trap_enter_soon),
	.trap_enter_vld             (m_trap_enter_vld),
	.trap_enter_rdy             (m_trap_enter_rdy),
	.loadstore_dphase_pending   (m_dphase_in_flight),
	.delay_irq_entry            (m_delay_irq_entry || d_uninterruptible),
	.mepc_in                    (m_exception_return_addr),

	.pwr_allow_clkgate          (m_pwr_allow_clkgate),
	.pwr_allow_power_down       (m_pwr_allow_power_down),
	.pwr_allow_sleep_on_block   (m_pwr_allow_sleep_on_block),
	.pwr_wfi_wakeup_req         (m_wfi_wakeup_req),

	.m_mode_execution           (x_mmode_execution),
	.m_mode_loadstore           (x_mmode_loadstore),
	.m_mode_trap_entry          (m_mmode_trap_entry),

	// IRQ and exception requests
	.irq                        (irq),
	.irq_software               (soft_irq),
	.irq_timer                  (timer_irq),
	.except                     (xm_except),
	.except_to_d_mode           (xm_except_to_d_mode),

	.pmp_cfg_addr               (x_pmp_cfg_addr),
	.pmp_cfg_wen                (x_pmp_cfg_wen),
	.pmp_cfg_wdata              (x_pmp_cfg_wdata),
	.pmp_cfg_rdata              (x_pmp_cfg_rdata),

	.trig_cfg_addr              (x_trig_cfg_addr),
	.trig_cfg_wen               (x_trig_cfg_wen),
	.trig_cfg_wdata             (x_trig_cfg_wdata),
	.trig_cfg_rdata             (x_trig_cfg_rdata),
	.trig_m_en                  (x_trig_m_en),

	.trig_event_interrupt       (m_trig_event_interrupt),
	.trig_event_exception       (m_trig_event_exception),
	.trig_event_trap_cause      (m_trig_event_trap_cause),

	// Other CSR-specific signalling
	.clear_excl_on_trap_exit    (m_clear_excl_on_trap_exit),
	.trap_wfi                   (x_trap_wfi),
	.instr_ret                  (x_instr_ret),
	.mhartid_val                (mhartid_val),
	.eco_version                (eco_version)
);

// Pipe register

assign xm_rd_nxt = REGADDR_MASK & (
	m_stall                                   ? xm_rd :
	x_stall || d_starved || m_trap_enter_soon ? 5'h0  : d_rd
);

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		xm_memop <= MEMOP_NONE;
		xm_except <= EXCEPT_NONE;
		xm_except_to_d_mode <= 1'b0;
		xm_sleep_wfi <= 1'b0;
		xm_sleep_block <= 1'b0;
		unblock_out <= 1'b0;
		xm_rs1 <= {W_REGADDR{1'b0}};
		xm_rs2 <= {W_REGADDR{1'b0}};
		xm_rd <= {W_REGADDR{1'b0}};
		xm_no_pc_increment <= 1'b0;
	end else begin
		unblock_out <= 1'b0;
		if (!m_stall) begin
			xm_rs1 <= REGADDR_MASK & d_rs1;
			xm_rs2 <= REGADDR_MASK & d_rs2;
			xm_rd  <= REGADDR_MASK & d_rd;
			// PC increment is suppressed non-final micro-ops, only needed for Zcmp:
			xm_no_pc_increment <= d_no_pc_increment && |EXTENSION_ZCMP;
			// If some X-sourced exception has squashed the address phase, need to squash the data phase too.
			xm_memop            <= x_except != EXCEPT_NONE ? MEMOP_NONE : d_memop;
			xm_except           <= x_except;
			xm_except_to_d_mode <= x_trig_break_d_mode;
			xm_sleep_wfi        <= x_except == EXCEPT_NONE && d_sleep_wfi;
			xm_sleep_block      <= x_except == EXCEPT_NONE && d_sleep_block;
			unblock_out         <= x_except == EXCEPT_NONE && d_sleep_unblock;
			// Note the d_starved term is required because it is possible
			// (e.g. PMP X permission fail) to except when the frontend is
			// starved, and we get a bad mepc if we let this jump ahead:
			if (x_stall || d_starved || m_trap_enter_soon) begin
				// Insert bubble
				xm_rd               <= {W_REGADDR{1'b0}};
				xm_memop            <= MEMOP_NONE;
				xm_except           <= EXCEPT_NONE;
				xm_except_to_d_mode <= 1'b0;
				xm_sleep_wfi        <= 1'b0;
				xm_sleep_block      <= 1'b0;
				unblock_out         <= 1'b0;
			end
		end else if (bus_dph_err_d) begin
			// First phase of 2-phase AHB5 error response. Pass the exception along on
			// this cycle, and on the next cycle the trap entry will be asserted,
			// suppressing any load/store that may currently be in stage X.
			xm_except <=
				|EXTENSION_A && xm_memop == MEMOP_LR_W ? EXCEPT_LOAD_FAULT :
				                xm_memop <= MEMOP_LBU  ? EXCEPT_LOAD_FAULT : EXCEPT_STORE_FAULT;
		end
	end
end


















































// Datapath flops
always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		xm_result <= {W_DATA{1'b0}};
		xm_addr_align <= 2'b00;
	end else if (!m_stall && !(|EXTENSION_A && x_amo_phase == 3'h3 && !bus_dph_ready_d)) begin
		// AMOs need special attention (of course):
		// - Steer captured read phase data in mw_result back through xm_result at end of AMO
		// - Make sure xm_result (store data) doesn't transition during stalled write dphase
		xm_result <=
			d_csr_ren                               ? x_csr_rdata       :
			|EXTENSION_A && x_amo_phase == 3'h3     ? mw_result         :
			|MUL_FASTER  && x_use_fast_mul          ? m_fast_mul_result :
			|EXTENSION_M && d_aluop == ALUOP_MULDIV ? x_muldiv_result   :
			                                          x_alu_result;
		xm_addr_align <= x_addr_sum[1:0];
	end
end

// ----------------------------------------------------------------------------
//                               Pipe Stage M

reg [W_DATA-1:0] m_rdata_pick_sext;
reg [W_DATA-1:0] m_wdata;
reg [W_DATA-1:0] m_result;

assign f_jump_req = x_jump_req || m_trap_enter_vld;
assign f_jump_target = m_trap_enter_vld	? m_trap_addr : x_jump_target;
assign f_jump_priv = m_trap_enter_vld ? m_mmode_trap_entry : x_mmode_execution;
assign x_jump_not_except = !m_trap_enter_vld;

// Stalls and sleep control

// EXCEPT_NONE clause is needed in the following sequence:
// - Cycle 0: hresp asserted, hready low. We set the exception to squash behind us. Bus stall high.
// - Cycle 1: hready high. For whatever reason, the frontend can't accept the trap address this cycle.
// - Cycle 2: Our dataphase has ended, so bus_dph_ready_d doesn't pulse again. m_bus_stall stuck high.
wire m_bus_stall = m_dphase_in_flight && !bus_dph_ready_d && xm_except == EXCEPT_NONE && !(
	|EXTENSION_A && xm_memop == MEMOP_SC_W && !mw_local_exclusive_reserved
);

assign m_stall = m_bus_stall ||
	(m_trap_enter_vld && !m_trap_enter_rdy && !m_trap_is_irq) ||
	((xm_sleep_wfi || xm_sleep_block) && !m_sleep_stall_release);

// Exception is taken against the instruction currently in M, so walk the PC
// back. IRQ is taken "in between" the instruction in M and the instruction
// in X, so set return to X program counter. Note that, if taking an
// exception, we know that the previous instruction to be in X (now in M)
// was *not* a taken branch, which is why we can just walk back the PC.
assign m_exception_return_addr = d_pc - (
	m_trap_is_irq         ? 32'd0 :
	xm_no_pc_increment    ? 32'd0 :
	prev_instr_was_32_bit ? 32'd4 : 32'd2
);

hazard3_power_ctrl power_ctrl (
	.clk_always_on          (clk_always_on),
	.rst_n                  (rst_n),

	.pwrup_req              (pwrup_req),
	.pwrup_ack              (pwrup_ack),
	.clk_en                 (clk_en),

	.allow_clkgate          (m_pwr_allow_clkgate),
	.allow_power_down       (m_pwr_allow_power_down),
	.allow_sleep_on_block   (m_pwr_allow_sleep_on_block),

	.frontend_pwrdown_ok    (f_frontend_pwrdown_ok),

	.sleeping_on_wfi        (xm_sleep_wfi),
	.wfi_wakeup_req         (m_wfi_wakeup_req),
	.sleeping_on_block      (xm_sleep_block),
	.block_wakeup_req_pulse (unblock_in),
	.stall_release          (m_sleep_stall_release)
);

// Load/store data handling

always @ (*) begin
	// Local forwarding of store data
	if (|mw_rd && xm_rs2 == mw_rd && !REDUCED_BYPASS) begin
		m_wdata = mw_result;
	end else begin
		m_wdata = xm_result;
	end
	// Replicate store data to ensure appropriate byte lane is driven
	case (xm_memop)
		MEMOP_SH: bus_wdata_d = {2{m_wdata[15:0]}};
		MEMOP_SB: bus_wdata_d = {4{m_wdata[7:0]}};
		default:  bus_wdata_d = m_wdata;
	endcase

	casez ({xm_memop, xm_addr_align[1:0]})
		{MEMOP_LH  , 2'b0z}: m_rdata_pick_sext = {{16{bus_rdata_d[15]}}, bus_rdata_d[15: 0]};
		{MEMOP_LH  , 2'b1z}: m_rdata_pick_sext = {{16{bus_rdata_d[31]}}, bus_rdata_d[31:16]};
		{MEMOP_LHU , 2'b0z}: m_rdata_pick_sext = {{16{1'b0           }}, bus_rdata_d[15: 0]};
		{MEMOP_LHU , 2'b1z}: m_rdata_pick_sext = {{16{1'b0           }}, bus_rdata_d[31:16]};
		{MEMOP_LB  , 2'b00}: m_rdata_pick_sext = {{24{bus_rdata_d[ 7]}}, bus_rdata_d[ 7: 0]};
		{MEMOP_LB  , 2'b01}: m_rdata_pick_sext = {{24{bus_rdata_d[15]}}, bus_rdata_d[15: 8]};
		{MEMOP_LB  , 2'b10}: m_rdata_pick_sext = {{24{bus_rdata_d[23]}}, bus_rdata_d[23:16]};
		{MEMOP_LB  , 2'b11}: m_rdata_pick_sext = {{24{bus_rdata_d[31]}}, bus_rdata_d[31:24]};
		{MEMOP_LBU , 2'b00}: m_rdata_pick_sext = {{24{1'b0           }}, bus_rdata_d[ 7: 0]};
		{MEMOP_LBU , 2'b01}: m_rdata_pick_sext = {{24{1'b0           }}, bus_rdata_d[15: 8]};
		{MEMOP_LBU , 2'b10}: m_rdata_pick_sext = {{24{1'b0           }}, bus_rdata_d[23:16]};
		{MEMOP_LBU , 2'b11}: m_rdata_pick_sext = {{24{1'b0           }}, bus_rdata_d[31:24]};
		{MEMOP_LW  , 2'bzz}: m_rdata_pick_sext = bus_rdata_d;
		{MEMOP_LR_W, 2'bzz}: m_rdata_pick_sext = bus_rdata_d;
		default:             m_rdata_pick_sext = 32'hxxxx_xxxx;
	endcase

	if (|EXTENSION_A && x_amo_phase == 3'h1) begin
		// Capture AMO read data into mw_result for feeding back through the ALU.
		m_result = bus_rdata_d;
	end else if (|EXTENSION_A && xm_memop == MEMOP_SC_W) begin
		// sc.w may fail due to negative response from either local or global monitor.
		// Note the polarity of the result: 0 for success, 1 for failure.
		m_result = {31'h0, !(mw_local_exclusive_reserved && bus_dph_exokay_d)};
	end else if (xm_memop != MEMOP_NONE && xm_memop != MEMOP_AMO) begin
		m_result = m_rdata_pick_sext;
	end else if (MUL_FAST && m_fast_mul_result_vld) begin
		m_result = m_fast_mul_result;
	end else begin
		m_result = xm_result;
	end
end

// Local monitor update.
// - Set on a load-reserved with good response from global monitor
// - Cleared by any store-conditional
// - Cleared by non-debug-related trap exit

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mw_local_exclusive_reserved <= 1'b0;
	end else begin
		if (|EXTENSION_A && (!m_stall || bus_dph_err_d)) begin
			if (d_memop_is_amo) begin
				mw_local_exclusive_reserved <= 1'b0;
			end else if (xm_memop == MEMOP_SC_W && (bus_dph_ready_d || bus_dph_err_d)) begin
				mw_local_exclusive_reserved <= 1'b0;
			end else if (xm_memop == MEMOP_LR_W && bus_dph_ready_d) begin
				// In theory, the bus should never report HEXOKAY when HRESP is asserted.
				// Still might happen (e.g. if HEXOKAY is tied high), so mask HEXOKAY with
				// HRESP to be sure a failed lr.w clears the monitor.
				mw_local_exclusive_reserved <= bus_dph_exokay_d && !bus_dph_err_d;
			end
		end
		if (m_clear_excl_on_trap_exit) begin
			mw_local_exclusive_reserved <= 1'b0;
		end
	end
end

// Note that exception entry prevents writeback, because the exception entry
// replaces the instruction in M. Interrupt entry does not prevent writeback,
// because the interrupt is notionally inserted in between the instruction in
// M and the instruction in X.

wire m_reg_wen_if_nonzero = !m_bus_stall && xm_except == EXCEPT_NONE;
wire m_reg_wen = |xm_rd && m_reg_wen_if_nonzero;

























always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mw_result <= {W_DATA{1'b0}};
	end else if (m_reg_wen_if_nonzero && !(|EXTENSION_A && x_amo_phase[1])) begin
		// (don't trash the captured AMO read phase data during stage 2/3 of AMO -- we need it!)
		mw_result <= m_result;
	end
end

assign mw_rd_nxt = REGADDR_MASK & (
	m_reg_wen_if_nonzero ? xm_rd : mw_rd
);

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mw_rd <= {W_REGADDR{1'b0}};
	end else begin






		if (m_reg_wen_if_nonzero) begin
			mw_rd <= REGADDR_MASK & xm_rd;
		end
	end
end

hazard3_regfile_1w2r #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
) regs (
	.clk    (clk),
	.rst_n  (rst_n),
	// On downstream stall, we feed D's addresses back into regfile
	// so that output does not change.
	.raddr1 (x_stall && !d_starved ? d_rs1 : f_rs1_coarse),
	.rdata1 (x_rdata1),
	.raddr2 (x_stall && !d_starved ? d_rs2 : f_rs2_coarse),
	.rdata2 (x_rdata2),

	.waddr  (xm_rd),
	.wdata  (m_result),
	.wen    (m_reg_wen)
);










endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Single-ported top level file for Hazard3 CPU. This file instantiates the
// Hazard3 core, and arbitrates its instruction fetch and load/store signals
// down to a single AHB5 master port.





`default_nettype none

module hazard3_cpu_1port #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
) (
	// Global signals
	input wire                clk,
	input wire                clk_always_on,
	input wire                rst_n,

	



	// Power control signals
	output wire               pwrup_req,
	input  wire               pwrup_ack,
	output wire               clk_en,
	output wire               unblock_out,
	input  wire               unblock_in,

	// AHB5 Master port
	output reg  [W_ADDR-1:0]  haddr,
	output reg                hwrite,
	output reg  [1:0]         htrans,
	output reg  [2:0]         hsize,
	output wire [2:0]         hburst,
	output reg  [3:0]         hprot,
	output wire               hmastlock,
	output reg  [7:0]         hmaster,
	output reg                hexcl,
	input  wire               hready,
	input  wire               hresp,
	input  wire               hexokay,
	output wire [W_DATA-1:0]  hwdata,
	input  wire [W_DATA-1:0]  hrdata,

	// Memory ordering signals
	output wire               fence_i_vld,
	output wire               fence_d_vld,
	input  wire               fence_rdy,

	// Debugger run/halt control
	input  wire               dbg_req_halt,
	input  wire               dbg_req_halt_on_reset,
	input  wire               dbg_req_resume,
	output wire               dbg_halted,
	output wire               dbg_running,
	// Debugger access to data0 CSR
	input  wire [W_DATA-1:0]  dbg_data0_rdata,
	output wire [W_DATA-1:0]  dbg_data0_wdata,
	output wire               dbg_data0_wen,
	// Debugger instruction injection
	input  wire [W_DATA-1:0]  dbg_instr_data,
	input  wire               dbg_instr_data_vld,
	output wire               dbg_instr_data_rdy,
	output wire               dbg_instr_caught_exception,
	output wire               dbg_instr_caught_ebreak,

	// Optional debug system bus access patch-through
	input  wire [W_ADDR-1:0]  dbg_sbus_addr,
	input  wire               dbg_sbus_write,
	input  wire [1:0]         dbg_sbus_size,
	input  wire               dbg_sbus_vld,
	output wire               dbg_sbus_rdy,
	output wire               dbg_sbus_err,
	input  wire [W_DATA-1:0]  dbg_sbus_wdata,
	output wire [W_DATA-1:0]  dbg_sbus_rdata,

	// Identification CSR values
	input  wire [W_DATA-1:0]  mhartid_val,
	input  wire [3:0]         eco_version,

	// Level-sensitive interrupt sources
	input wire [NUM_IRQS-1:0] irq,       // -> mip.meip
	input wire                soft_irq,  // -> mip.msip
	input wire                timer_irq  // -> mip.mtip
);

// ----------------------------------------------------------------------------
// Processor core

// Instruction fetch signals
wire              core_aph_req_i;
wire              core_aph_panic_i;
wire              core_aph_ready_i;
wire              core_dph_ready_i;
wire              core_dph_err_i;

wire [W_ADDR-1:0] core_haddr_i;
wire [2:0]        core_hsize_i;
wire              core_priv_i;
wire [W_DATA-1:0] core_rdata_i;


// Load/store signals
wire              core_aph_req_d;
wire              core_aph_excl_d;
wire              core_aph_ready_d;
wire              core_dph_ready_d;
wire              core_dph_err_d;
wire              core_dph_exokay_d;

wire [W_ADDR-1:0] core_haddr_d;
wire [2:0]        core_hsize_d;
wire              core_priv_d;
wire              core_hwrite_d;
wire [W_DATA-1:0] core_wdata_d;
wire [W_DATA-1:0] core_rdata_d;


hazard3_core #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
) core (
	.clk                        (clk),
	.clk_always_on              (clk_always_on),
	.rst_n                      (rst_n),

	.pwrup_req                  (pwrup_req),
	.pwrup_ack                  (pwrup_ack),
	.clk_en                     (clk_en),
	.unblock_out                (unblock_out),
	.unblock_in                 (unblock_in),

	



	.bus_aph_req_i              (core_aph_req_i),
	.bus_aph_panic_i            (core_aph_panic_i),
	.bus_aph_ready_i            (core_aph_ready_i),
	.bus_dph_ready_i            (core_dph_ready_i),
	.bus_dph_err_i              (core_dph_err_i),
	.bus_haddr_i                (core_haddr_i),
	.bus_hsize_i                (core_hsize_i),
	.bus_priv_i                 (core_priv_i),
	.bus_rdata_i                (core_rdata_i),

	.bus_aph_req_d              (core_aph_req_d),
	.bus_aph_excl_d             (core_aph_excl_d),
	.bus_aph_ready_d            (core_aph_ready_d),
	.bus_dph_ready_d            (core_dph_ready_d),
	.bus_dph_err_d              (core_dph_err_d),
	.bus_dph_exokay_d           (core_dph_exokay_d),
	.bus_haddr_d                (core_haddr_d),
	.bus_hsize_d                (core_hsize_d),
	.bus_priv_d                 (core_priv_d),
	.bus_hwrite_d               (core_hwrite_d),
	.bus_wdata_d                (core_wdata_d),
	.bus_rdata_d                (core_rdata_d),

	.fence_i_vld                (fence_i_vld),
	.fence_d_vld                (fence_d_vld),
	.fence_rdy                  (fence_rdy),

	.dbg_req_halt               (dbg_req_halt),
	.dbg_req_halt_on_reset      (dbg_req_halt_on_reset),
	.dbg_req_resume             (dbg_req_resume),
	.dbg_halted                 (dbg_halted),
	.dbg_running                (dbg_running),
	.dbg_data0_rdata            (dbg_data0_rdata),
	.dbg_data0_wdata            (dbg_data0_wdata),
	.dbg_data0_wen              (dbg_data0_wen),
	.dbg_instr_data             (dbg_instr_data),
	.dbg_instr_data_vld         (dbg_instr_data_vld),
	.dbg_instr_data_rdy         (dbg_instr_data_rdy),
	.dbg_instr_caught_exception (dbg_instr_caught_exception),
	.dbg_instr_caught_ebreak    (dbg_instr_caught_ebreak),

	.mhartid_val                (mhartid_val),
	.eco_version                (eco_version),

	.irq                        (irq),
	.soft_irq                   (soft_irq),
	.timer_irq                  (timer_irq)
);

// ----------------------------------------------------------------------------
// Arbitration state machine

wire      bus_gnt_i;
wire      bus_gnt_d;
wire      bus_gnt_s;

reg       bus_hold_aph;
reg [2:0] bus_gnt_ids_prev;

// Note use of clk_always_on: SBA may use this arbiter to access the bus
// whilst the core is asleep.

always @ (posedge clk_always_on or negedge rst_n) begin
	if (!rst_n) begin
		bus_hold_aph <= 1'b0;
		bus_gnt_ids_prev <= 3'h0;
	end else begin
		bus_hold_aph <= htrans[1] && !hready && !hresp;
		bus_gnt_ids_prev <= {bus_gnt_i, bus_gnt_d, bus_gnt_s};
	end
end

// Debug SBA access is lower priority than load/store, but higher than
// instruction fetch. This isn't ideal, but in a tight loop the core may be
// performing an instruction fetch or load/store on every single cycle, and
// this is a simple way to guarantee eventual success of debugger accesses. A
// more complex way would be to add a "panic timer" to boost a stalled sbus
// access over an instruction fetch.

// Note that, often, the sbus will be disconnected: it doesn't provide any
// increase in debugger bus throughput compared with the program buffer and
// autoexec. It's useful for "minimally intrusive" debug bus access(i.e. less
// intrusive than halting the core and resuming it) e.g. for Segger RTT.

reg bus_active_dph_s;

assign {bus_gnt_i, bus_gnt_d, bus_gnt_s} =
	bus_hold_aph                      ? bus_gnt_ids_prev :
	core_aph_panic_i                  ? 3'b100 :
	core_aph_req_d                    ? 3'b010 :
	dbg_sbus_vld && !bus_active_dph_s ? 3'b001 :
	core_aph_req_i                    ? 3'b100 :
	                                    3'b000 ;
reg bus_active_dph_i;
reg bus_active_dph_d;

always @ (posedge clk_always_on or negedge rst_n) begin
	if (!rst_n) begin
		bus_active_dph_i <= 1'b0;
		bus_active_dph_d <= 1'b0;
		bus_active_dph_s <= 1'b0;
	end else if (hready) begin
		bus_active_dph_i <= bus_gnt_i;
		bus_active_dph_d <= bus_gnt_d;
		bus_active_dph_s <= bus_gnt_s;
	end
end

// ----------------------------------------------------------------------------
// Address phase request muxing

localparam HTRANS_IDLE = 2'b00;
localparam HTRANS_NSEQ = 2'b10;

wire [3:0] hprot_data  = {
	2'b00,                  // Noncacheable/nonbufferable
	core_priv_d,            // Privileged or Normal as per core state
	1'b1                    // Data access
};

wire [3:0] hprot_instr = {
	2'b00,                  // Noncacheable/nonbufferable
	core_priv_i,            // Privileged or Normal as per core state
	1'b0                    // Instruction access
};

wire [3:0] hprot_sbus = {
	2'b00,         // Noncacheable/nonbufferable
	1'b1,          // Always privileged
	1'b1           // Data access
};

assign hburst = 3'b000;   // HBURST_SINGLE
assign hmastlock = 1'b0;

always @ (*) begin
	if (bus_gnt_s) begin
		htrans  = HTRANS_NSEQ;
		hexcl   = 1'b0;
		haddr   = dbg_sbus_addr;
		hsize   = {1'b0, dbg_sbus_size};
		hwrite  = dbg_sbus_write;
		hprot   = hprot_sbus;
		hmaster = 8'h01;
	end else if (bus_gnt_d) begin
		htrans  = HTRANS_NSEQ;
		hexcl   = core_aph_excl_d;
		haddr   = core_haddr_d;
		hsize   = core_hsize_d;
		hwrite  = core_hwrite_d;
		hprot   = hprot_data;
		hmaster = 8'h00;
	end else if (bus_gnt_i) begin
		htrans  = HTRANS_NSEQ;
		hexcl   = 1'b0;
		haddr   = core_haddr_i;
		hsize   = core_hsize_i;
		hwrite  = 1'b0;
		hprot   = hprot_instr;
		hmaster = 8'h00;
	end else begin
		htrans  = HTRANS_IDLE;
		hexcl   = 1'b0;
		haddr   = {W_ADDR{1'b0}};
		hsize   = 3'h0;
		hwrite  = 1'b0;
		hprot   = 4'h0;
		hmaster = 8'h00;
	end
end

assign hwdata = bus_active_dph_s ? dbg_sbus_wdata : core_wdata_d;

// ----------------------------------------------------------------------------
// Response routing

// Data buses directly connected
assign core_rdata_d   = hrdata;
assign core_rdata_i   = hrdata;
assign dbg_sbus_rdata = hrdata;

// Handhshake based on grant and bus stall
assign core_aph_ready_i = hready && bus_gnt_i;
assign core_dph_ready_i = bus_active_dph_i && hready;
assign core_dph_err_i   = bus_active_dph_i && hresp;

// D-side errors are reported even when not ready, so that the core can make
// use of the two-phase error response to cleanly squash a second load/store
// chasing the faulting one down the pipeline.
assign core_aph_ready_d  = hready && bus_gnt_d;
assign core_dph_ready_d  = bus_active_dph_d && hready;
assign core_dph_err_d    = bus_active_dph_d && hresp;
assign core_dph_exokay_d = bus_active_dph_d && hexokay;

assign dbg_sbus_err = bus_active_dph_s && hresp;
assign dbg_sbus_rdy = bus_active_dph_s && hready;

endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Dual-ported top level file for Hazard3 CPU. This file instantiates the
// Hazard3 core, and interfaces its instruction fetch and load/store signals
// to a pair of AHB5 master ports.





`default_nettype none

module hazard3_cpu_2port #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
) (
	// Global signals
	input wire                clk,
	input wire                clk_always_on,
	input wire                rst_n,

	// Power control signals
	output wire               pwrup_req,
	input  wire               pwrup_ack,
	output wire               clk_en,
	output wire               unblock_out,
	input  wire               unblock_in,

	



	// Instruction fetch port
	output wire [W_ADDR-1:0]  i_haddr,
	output wire               i_hwrite,
	output wire [1:0]         i_htrans,
	output wire [2:0]         i_hsize,
	output wire [2:0]         i_hburst,
	output wire [3:0]         i_hprot,
	output wire               i_hmastlock,
	output wire [7:0]         i_hmaster,
	input  wire               i_hready,
	input  wire               i_hresp,
	output wire [W_DATA-1:0]  i_hwdata,
	input  wire [W_DATA-1:0]  i_hrdata,

	// Load/store port
	output wire [W_ADDR-1:0]  d_haddr,
	output wire               d_hwrite,
	output wire [1:0]         d_htrans,
	output wire [2:0]         d_hsize,
	output wire [2:0]         d_hburst,
	output wire [3:0]         d_hprot,
	output wire               d_hmastlock,
	output wire [7:0]         d_hmaster,
	output wire               d_hexcl,
	input  wire               d_hready,
	input  wire               d_hresp,
	input  wire               d_hexokay,
	output wire [W_DATA-1:0]  d_hwdata,
	input  wire [W_DATA-1:0]  d_hrdata,

	// Memory ordering signals
	output wire               fence_i_vld,
	output wire               fence_d_vld,
	input  wire               fence_rdy,

	// Debugger run/halt control
	input  wire               dbg_req_halt,
	input  wire               dbg_req_halt_on_reset,
	input  wire               dbg_req_resume,
	output wire               dbg_halted,
	output wire               dbg_running,
	// Debugger access to data0 CSR
	input  wire [W_DATA-1:0]  dbg_data0_rdata,
	output wire [W_DATA-1:0]  dbg_data0_wdata,
	output wire               dbg_data0_wen,
	// Debugger instruction injection
	input  wire [W_DATA-1:0]  dbg_instr_data,
	input  wire               dbg_instr_data_vld,
	output wire               dbg_instr_data_rdy,
	output wire               dbg_instr_caught_exception,
	output wire               dbg_instr_caught_ebreak,
	// Optional debug system bus access patch-through
	input  wire [W_ADDR-1:0]  dbg_sbus_addr,
	input  wire               dbg_sbus_write,
	input  wire [1:0]         dbg_sbus_size,
	input  wire               dbg_sbus_vld,
	output wire               dbg_sbus_rdy,
	output wire               dbg_sbus_err,
	input  wire [W_DATA-1:0]  dbg_sbus_wdata,
	output wire [W_DATA-1:0]  dbg_sbus_rdata,

	// Identification CSR values
	input  wire [W_DATA-1:0]  mhartid_val,
	input  wire [3:0]         eco_version,

	// Level-sensitive interrupt sources
	input wire [NUM_IRQS-1:0] irq,       // -> mip.meip
	input wire                soft_irq,  // -> mip.msip
	input wire                timer_irq  // -> mip.mtip
);

// ----------------------------------------------------------------------------
// Processor core

// Instruction fetch signals
wire              core_aph_req_i;
wire              core_aph_ready_i;
wire              core_dph_ready_i;
wire              core_dph_err_i;

wire [W_ADDR-1:0] core_haddr_i;
wire [2:0]        core_hsize_i;
wire              core_priv_i;
wire [W_DATA-1:0] core_rdata_i;


// Load/store signals
wire              core_aph_req_d;
wire              core_aph_excl_d;
wire              core_aph_ready_d;
wire              core_dph_ready_d;
wire              core_dph_err_d;
wire              core_dph_exokay_d;

wire [W_ADDR-1:0] core_haddr_d;
wire [2:0]        core_hsize_d;
wire              core_priv_d;
wire              core_hwrite_d;
wire [W_DATA-1:0] core_wdata_d;
wire [W_DATA-1:0] core_rdata_d;

hazard3_core #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
) core (
	.clk                        (clk),
	.clk_always_on              (clk_always_on),
	.rst_n                      (rst_n),

	.pwrup_req                  (pwrup_req),
	.pwrup_ack                  (pwrup_ack),
	.clk_en                     (clk_en),
	.unblock_out                (unblock_out),
	.unblock_in                 (unblock_in),

	



	.bus_aph_req_i              (core_aph_req_i),
	.bus_aph_panic_i            (/* unused for 2port */),
	.bus_aph_ready_i            (core_aph_ready_i),
	.bus_dph_ready_i            (core_dph_ready_i),
	.bus_dph_err_i              (core_dph_err_i),
	.bus_haddr_i                (core_haddr_i),
	.bus_hsize_i                (core_hsize_i),
	.bus_priv_i                 (core_priv_i),
	.bus_rdata_i                (core_rdata_i),

	.bus_aph_req_d              (core_aph_req_d),
	.bus_aph_excl_d             (core_aph_excl_d),
	.bus_aph_ready_d            (core_aph_ready_d),
	.bus_dph_ready_d            (core_dph_ready_d),
	.bus_dph_err_d              (core_dph_err_d),
	.bus_dph_exokay_d           (core_dph_exokay_d),
	.bus_haddr_d                (core_haddr_d),
	.bus_hsize_d                (core_hsize_d),
	.bus_priv_d                 (core_priv_d),
	.bus_hwrite_d               (core_hwrite_d),
	.bus_wdata_d                (core_wdata_d),
	.bus_rdata_d                (core_rdata_d),

	.fence_i_vld                (fence_i_vld),
	.fence_d_vld                (fence_d_vld),
	.fence_rdy                  (fence_rdy),

	.dbg_req_halt               (dbg_req_halt),
	.dbg_req_halt_on_reset      (dbg_req_halt_on_reset),
	.dbg_req_resume             (dbg_req_resume),
	.dbg_halted                 (dbg_halted),
	.dbg_running                (dbg_running),
	.dbg_data0_rdata            (dbg_data0_rdata),
	.dbg_data0_wdata            (dbg_data0_wdata),
	.dbg_data0_wen              (dbg_data0_wen),
	.dbg_instr_data             (dbg_instr_data),
	.dbg_instr_data_vld         (dbg_instr_data_vld),
	.dbg_instr_data_rdy         (dbg_instr_data_rdy),
	.dbg_instr_caught_exception (dbg_instr_caught_exception),
	.dbg_instr_caught_ebreak    (dbg_instr_caught_ebreak),

	.mhartid_val                (mhartid_val),
	.eco_version                (eco_version),

	.irq                        (irq),
	.soft_irq                   (soft_irq),
	.timer_irq                  (timer_irq)
);

// ----------------------------------------------------------------------------
// Instruction port

localparam HTRANS_IDLE = 2'b00;
localparam HTRANS_NSEQ = 2'b10;

assign i_haddr = core_haddr_i;
assign i_htrans = core_aph_req_i ? HTRANS_NSEQ : HTRANS_IDLE;
assign i_hsize = core_hsize_i;

reg dphase_active_i;
always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		dphase_active_i <= 1'b0;
	end else if (i_hready) begin
		dphase_active_i <= core_aph_req_i;
	end
end







assign core_aph_ready_i = i_hready && core_aph_req_i;
assign core_dph_ready_i = i_hready && dphase_active_i;
assign core_dph_err_i   = i_hready && dphase_active_i && i_hresp;

assign core_rdata_i = i_hrdata;

assign i_hwrite = 1'b0;
assign i_hburst = 3'h0;
assign i_hmastlock = 1'b0;
assign i_hmaster = 8'h00;
assign i_hwdata = {W_DATA{1'b0}};

assign i_hprot = {
	2'b00,         // Noncacheable/nonbufferable
	core_priv_i,   // Privileged or Normal as per core state
	1'b0           // Instruction access
};

// ----------------------------------------------------------------------------
// Load/store port

// The debug module has optional System Bus Access support, which can be muxed
// into the processor's D port here (or connected to a standalone AHB shim).
// This confers absolutely no advantage for debugger bus throughput, but
// allows the debugger to access the bus with minimal disturbance to the
// processor.

wire      bus_gnt_d;
wire      bus_gnt_s;

reg       bus_hold_aph;
reg [1:0] bus_gnt_ds_prev;
reg       bus_active_dph_d;
reg       bus_active_dph_s;

// clk_always_on is used because SBA may access the bus through this arbiter
// whilst the core is asleep (same is not true for I-side interface)

always @ (posedge clk_always_on or negedge rst_n) begin
	if (!rst_n) begin
		bus_hold_aph <= 1'b0;
		bus_gnt_ds_prev <= 2'h0;
	end else begin
		bus_hold_aph <= d_htrans[1] && !d_hready && !d_hresp;
		bus_gnt_ds_prev <= {bus_gnt_d, bus_gnt_s};
	end
end

assign {bus_gnt_d, bus_gnt_s} =
	bus_hold_aph                      ? bus_gnt_ds_prev :
	core_aph_req_d                    ? 2'b10 :
	dbg_sbus_vld && !bus_active_dph_s ? 2'b01 :
	                                    2'b00 ;

always @ (posedge clk_always_on or negedge rst_n) begin
	if (!rst_n) begin
		bus_active_dph_d <= 1'b0;
		bus_active_dph_s <= 1'b0;
	end else if (d_hready) begin
		bus_active_dph_d <= bus_gnt_d;
		bus_active_dph_s <= bus_gnt_s;
	end
end

assign d_htrans = bus_gnt_d || bus_gnt_s ? HTRANS_NSEQ : HTRANS_IDLE;

assign d_haddr  = bus_gnt_s ? dbg_sbus_addr         : core_haddr_d;
assign d_hwrite = bus_gnt_s ? dbg_sbus_write        : core_hwrite_d;
assign d_hsize  = bus_gnt_s ? {1'b0, dbg_sbus_size} : core_hsize_d;
assign d_hexcl  = bus_gnt_s ? 1'b0                  : core_aph_excl_d;

assign d_hprot = {
	2'b00,                    // Noncacheable/nonbufferable
	bus_gnt_s || core_priv_d, // Privileged or Normal as per core state
	1'b1                      // Data access
};

assign d_hwdata = bus_active_dph_s ? dbg_sbus_wdata : core_wdata_d;

// D-side errors are reported even when not ready, so that the core can make
// use of the two-phase error response to cleanly squash a second load/store
// chasing the faulting one down the pipeline.
assign core_aph_ready_d  = d_hready && bus_gnt_d;
assign core_dph_ready_d  = bus_active_dph_d && d_hready;
assign core_dph_err_d    = bus_active_dph_d && d_hresp;
assign core_dph_exokay_d = bus_active_dph_d && d_hexokay;
assign core_rdata_d = d_hrdata;

assign dbg_sbus_err = bus_active_dph_s && d_hresp;
assign dbg_sbus_rdy = bus_active_dph_s && d_hready;
assign dbg_sbus_rdata = d_hrdata;

assign d_hburst = 3'h0;
assign d_hmastlock = 1'b0;
assign d_hmaster = bus_gnt_s ? 8'h01 : 8'h00;

endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

`default_nettype none

module hazard3_alu #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
,
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// These really ought to be localparams, but are occasionally needed for
// passing flags around between modules, so are made available as parameters
// instead. It's ugly, but better scope hygiene than the preprocessor. These
// parameters should not be changed from their default values.

parameter W_REGADDR = 5,

parameter W_ALUOP   = 6,
parameter W_ALUSRC  = 1,
parameter W_MEMOP   = 5,
parameter W_BCOND   = 2,
parameter W_SHAMT   = 5,

parameter W_EXCEPT  = 4,
parameter W_MULOP   = 3
) (
	input  wire [W_ALUOP-1:0] aluop,
	input  wire [6:0]         funct7_32b,
	input  wire [2:0]         funct3_32b,
	input  wire [W_DATA-1:0]  op_a,
	input  wire [W_DATA-1:0]  op_b,
	output reg  [W_DATA-1:0]  result,
	output wire               cmp
);

/*****************************************************************************\
|                        Copyright (C) 2021 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// ALU operation selectors

localparam ALUOP_ADD     = 6'h00; 
localparam ALUOP_SUB     = 6'h01; 
localparam ALUOP_LT      = 6'h02;
localparam ALUOP_LTU     = 6'h04;
localparam ALUOP_AND     = 6'h06;
localparam ALUOP_OR      = 6'h07;
localparam ALUOP_XOR     = 6'h08;
localparam ALUOP_SRL     = 6'h09;
localparam ALUOP_SRA     = 6'h0a;
localparam ALUOP_SLL     = 6'h0b;
localparam ALUOP_MULDIV  = 6'h0c;
localparam ALUOP_RS2     = 6'h0d; // differs from AND/OR/XOR in [1:0]
// Bitmanip ALU operations (some also used by AMOs):
localparam ALUOP_SHXADD  = 6'h20;
localparam ALUOP_CLZ     = 6'h23;
localparam ALUOP_CPOP    = 6'h24;
localparam ALUOP_CTZ     = 6'h25;
localparam ALUOP_ANDN    = 6'h26; // Same LSBs as non-inverted
localparam ALUOP_ORN     = 6'h27; // Same LSBs as non-inverted
localparam ALUOP_XNOR    = 6'h28; // Same LSBs as non-inverted
localparam ALUOP_MAX     = 6'h29;
localparam ALUOP_MAXU    = 6'h2a;
localparam ALUOP_MIN     = 6'h2b;
localparam ALUOP_MINU    = 6'h2c;
localparam ALUOP_ORC_B   = 6'h2d;
localparam ALUOP_REV8    = 6'h2e;
localparam ALUOP_ROL     = 6'h2f;
localparam ALUOP_ROR     = 6'h30;
localparam ALUOP_SEXT_B  = 6'h31;
localparam ALUOP_SEXT_H  = 6'h32;
localparam ALUOP_ZEXT_H  = 6'h33;

localparam ALUOP_CLMUL   = 6'h34;

localparam ALUOP_BCLR    = 6'h35;
localparam ALUOP_BEXT    = 6'h36;
localparam ALUOP_BINV    = 6'h37;
localparam ALUOP_BSET    = 6'h38;

localparam ALUOP_PACK    = 6'h39;
localparam ALUOP_PACKH   = 6'h3a;
localparam ALUOP_BREV8   = 6'h3b;
localparam ALUOP_ZIP     = 6'h3c;
localparam ALUOP_UNZIP   = 6'h3d;

localparam ALUOP_BEXTM   = 6'h3e;

localparam ALUOP_XPERM   = 6'h3f;

// Parameters to control ALU input muxes. Bypass mux paths are
// controlled by X, so D has no parameters to choose these.

localparam ALUSRCA_RS1 = 1'h0;
localparam ALUSRCA_PC  = 1'h1;

localparam ALUSRCB_RS2 = 1'h0;
localparam ALUSRCB_IMM = 1'h1;

localparam MEMOP_LW   = 5'h00;
localparam MEMOP_LH   = 5'h01;
localparam MEMOP_LB   = 5'h02;
localparam MEMOP_LHU  = 5'h03;
localparam MEMOP_LBU  = 5'h04;
localparam MEMOP_SW   = 5'h05;
localparam MEMOP_SH   = 5'h06;
localparam MEMOP_SB   = 5'h07;

localparam MEMOP_LR_W = 5'h08;
localparam MEMOP_SC_W = 5'h09;
localparam MEMOP_AMO  = 5'h0a;
localparam MEMOP_NONE = 5'h10;

localparam BCOND_NEVER  = 2'h0;
localparam BCOND_ALWAYS = 2'h1;
localparam BCOND_ZERO   = 2'h2;
localparam BCOND_NZERO  = 2'h3;

// CSR access types

localparam CSR_WTYPE_W    = 2'h0;
localparam CSR_WTYPE_S    = 2'h1;
localparam CSR_WTYPE_C    = 2'h2;

// Exceptional condition signals which travel alongside (or instead of)
// instructions in the pipeline. These are speculative and can be flushed on
// e.g. branch mispredict. These mostly align with mcause values.

localparam EXCEPT_NONE           = 4'hf;

localparam EXCEPT_INSTR_MISALIGN = 4'h0;
localparam EXCEPT_INSTR_FAULT    = 4'h1;
localparam EXCEPT_INSTR_ILLEGAL  = 4'h2;
localparam EXCEPT_EBREAK         = 4'h3;
localparam EXCEPT_LOAD_ALIGN     = 4'h4;
localparam EXCEPT_LOAD_FAULT     = 4'h5;
localparam EXCEPT_STORE_ALIGN    = 4'h6;
localparam EXCEPT_STORE_FAULT    = 4'h7;
localparam EXCEPT_ECALL_U        = 4'h8;
// MRET, Return from M-mode: not really an exception, but handled like one
localparam EXCEPT_MRET           = 4'ha;
localparam EXCEPT_ECALL_M        = 4'hb;
// spare: c
// spare: d
// REFETCH: flush and refetch sequentially-following instructions, e.g. on
// executing fence.i. Jumps from stage 3 to get ordering against L/S dphase.
localparam EXCEPT_REFETCH        = 4'he;

// Operations for M extension (these are just instr[14:12])

localparam M_OP_MUL    = 3'h0;
localparam M_OP_MULH   = 3'h1;
localparam M_OP_MULHSU = 3'h2;
localparam M_OP_MULHU  = 3'h3;
localparam M_OP_DIV    = 3'h4;
localparam M_OP_DIVU   = 3'h5;
localparam M_OP_REM    = 3'h6;
localparam M_OP_REMU   = 3'h7;

// ----------------------------------------------------------------------------
// Fiddle around with add/sub, comparisons etc (all related).

wire sub = !(aluop == ALUOP_ADD || (|EXTENSION_ZBA && aluop == ALUOP_SHXADD));

wire inv_op_b = sub && !(
	aluop == ALUOP_AND || aluop == ALUOP_OR || aluop == ALUOP_XOR || aluop == ALUOP_RS2
);

wire [W_DATA-1:0] op_a_shifted =
	|EXTENSION_ZBA && aluop == ALUOP_SHXADD ? (
		!funct3_32b[2] ? op_a << 1 :
		!funct3_32b[1] ? op_a << 2 : op_a << 3
	) : op_a;

wire [W_DATA-1:0] op_b_inv = op_b ^ {W_DATA{inv_op_b}};

wire [W_DATA-1:0] sum  = op_a_shifted + op_b_inv + {{W_DATA-1{1'b0}}, sub};
wire [W_DATA-1:0] op_xor = op_a ^ op_b;

wire cmp_is_unsigned = aluop == ALUOP_LTU ||
	|EXTENSION_ZBB && aluop == ALUOP_MAXU ||
	|EXTENSION_ZBB && aluop == ALUOP_MINU;

wire lt = op_a[W_DATA-1] == op_b[W_DATA-1] ? sum[W_DATA-1]  :
          cmp_is_unsigned                  ? op_b[W_DATA-1] : op_a[W_DATA-1] ;

assign cmp = aluop == ALUOP_SUB ? |op_xor : lt;

// ----------------------------------------------------------------------------
// Separate units for shift, ctz etc

wire [W_DATA-1:0] shift_dout;
wire shift_right_nleft =
	aluop == ALUOP_SRL ||
	aluop == ALUOP_SRA ||
	(|EXTENSION_ZBB      && aluop == ALUOP_ROR  ) ||
	(|EXTENSION_ZBS      && aluop == ALUOP_BEXT ) ||
	(|EXTENSION_XH3BEXTM && aluop == ALUOP_BEXTM);

wire shift_arith = aluop == ALUOP_SRA;
wire shift_rotate = |EXTENSION_ZBB & (aluop == ALUOP_ROR || aluop == ALUOP_ROL);

hazard3_shift_barrel #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
) shifter (
	.din         (op_a),
	.shamt       (op_b[4:0]),
	.right_nleft (shift_right_nleft),
	.rotate      (shift_rotate),
	.arith       (shift_arith),
	.dout        (shift_dout)
);

reg [W_DATA-1:0] op_a_rev;
always @ (*) begin: rev_op_a
	integer i;
	for (i = 0; i < W_DATA; i = i + 1) begin
		op_a_rev[i] = op_a[W_DATA - 1 - i];
	end
end

// "leading" means starting at MSB. This is an LSB-first priority encoder, so
// "leading" is reversed and "trailing" is not.
wire [W_DATA-1:0] ctz_search_mask = aluop == ALUOP_CLZ ? op_a_rev : op_a;
wire [W_SHAMT:0]  ctz_clz;

hazard3_priority_encode #(
	.W_REQ        (W_DATA),
	.HIGHEST_WINS (0)
) ctz_priority_encode (
	.req (ctz_search_mask),
	.gnt (ctz_clz[W_SHAMT-1:0])
);
// Special case: all-zeroes returns XLEN
assign ctz_clz[W_SHAMT] = ~|op_a;

reg [W_SHAMT:0] cpop;
always @ (*) begin: cpop_count
	integer i;
	cpop = {W_SHAMT+1{1'b0}};
	for (i = 0; i < W_DATA; i = i + 1) begin
		cpop = cpop + {{W_SHAMT{1'b0}}, op_a[i]};
	end
end

reg [2*W_DATA-1:0] clmul64;

always @ (*) begin: clmul_mul
	integer i;
	clmul64 = {2*W_DATA{1'b0}};
	for (i = 0; i < W_DATA; i = i + 1) begin
		clmul64 = clmul64 ^ (({{W_DATA{1'b0}}, op_a} << i) & {2*W_DATA{op_b[i]}});
	end
end

// funct3: 1=clmul, 2=clmulr, 3=clmulh, never 0.
wire [W_DATA-1:0] clmul =
	!funct3_32b[1] ? clmul64[31: 0] :
	!funct3_32b[0] ? clmul64[62:31] : clmul64[63:32];

reg [W_DATA-1:0] zip;
reg [W_DATA-1:0] unzip;
always @ (*) begin: do_zip_unzip
	integer i;
	for (i = 0; i < W_DATA; i = i + 1) begin
		zip[i]   = op_a[{i[0], i[4:1]}]; // Alternate high/low halves
		unzip[i] = op_a[{i[3:0], i[4]}]; // All even then all odd
	end
end

reg [W_DATA-1:0] xperm8;
always @ (*) begin: do_xperm8
	integer i;
	for (i = 0; i < W_DATA; i = i + 8) begin
		if (|op_b[i + 2 +: 6]) begin
			xperm8[i +: 8] = 8'h00;
		end else begin
			xperm8[i +: 8] = op_a[8 * op_b[i +: 2] +: 8];
		end
	end
end

reg [W_DATA-1:0] xperm4;
always @ (*) begin: do_xperm4
	integer i;
	for (i = 0; i < W_DATA; i = i + 4) begin
		if (op_b[i + 3]) begin
			xperm4[i +: 4] = 4'h0;
		end else begin
			xperm4[i +: 4] = op_a[4 * op_b[i +: 3] +: 4];
		end
	end
end

// ----------------------------------------------------------------------------
// Output mux, with simple operations inline

// iCE40: We can implement all bitwise ops with 1 LUT4/bit total, since each
// result bit uses only two operand bits. Much better than feeding each into
// main mux tree. Doesn't matter for big-LUT FPGAs or for implementations with
// bitmanip extensions enabled.

reg [W_DATA-1:0] bitwise;

always @ (*) begin: bitwise_ops
	case (aluop[1:0])
		ALUOP_AND[1:0]: bitwise = op_a & op_b_inv;
		ALUOP_OR [1:0]: bitwise = op_a | op_b_inv;
		ALUOP_XOR[1:0]: bitwise = op_a ^ op_b_inv;
		ALUOP_RS2[1:0]: bitwise =        op_b_inv;
	endcase
end

wire [W_DATA-1:0] zbs_mask = {{W_DATA-1{1'b0}}, 1'b1} << op_b[W_SHAMT-1:0];

always @ (*) begin
	casez ({|EXTENSION_A, |EXTENSION_ZBA, |EXTENSION_ZBB, |EXTENSION_ZBC,
	        |EXTENSION_ZBS, |EXTENSION_ZBKB, |EXTENSION_ZBKX, |EXTENSION_XH3BEXTM, aluop})
		// Base ISA
		{8'bzzzzzzzz, ALUOP_ADD    }: result = sum;
		{8'bzzzzzzzz, ALUOP_SUB    }: result = sum;
		{8'bzzzzzzzz, ALUOP_LT     }: result = {{W_DATA-1{1'b0}}, lt};
		{8'bzzzzzzzz, ALUOP_LTU    }: result = {{W_DATA-1{1'b0}}, lt};
		{8'bzzzzzzzz, ALUOP_SRL    }: result = shift_dout;
		{8'bzzzzzzzz, ALUOP_SRA    }: result = shift_dout;
		{8'bzzzzzzzz, ALUOP_SLL    }: result = shift_dout;
		// A or Zbb (written this way to avoid case overlap)
		{8'b1zzzzzzz, ALUOP_MAX    },
		{8'b0z1zzzzz, ALUOP_MAX    }: result = lt ? op_b : op_a;
		{8'b1zzzzzzz, ALUOP_MIN    },
		{8'b0z1zzzzz, ALUOP_MIN    }: result = lt ? op_a : op_b;
		{8'b1zzzzzzz, ALUOP_MAXU   },
		{8'b0z1zzzzz, ALUOP_MAXU   }: result = lt ? op_b : op_a;
		{8'b1zzzzzzz, ALUOP_MINU   },
		{8'b0z1zzzzz, ALUOP_MINU   }: result = lt ? op_a : op_b;
		// Zba
		{8'bz1zzzzzz, ALUOP_SHXADD }: result = sum;
		// Zbb
		{8'bzz1zzzzz, ALUOP_ANDN   }: result = bitwise;
		{8'bzz1zzzzz, ALUOP_ORN    }: result = bitwise;
		{8'bzz1zzzzz, ALUOP_XNOR   }: result = bitwise;
		{8'bzz1zzzzz, ALUOP_CLZ    }: result = {{W_DATA-W_SHAMT-1{1'b0}}, ctz_clz};
		{8'bzz1zzzzz, ALUOP_CTZ    }: result = {{W_DATA-W_SHAMT-1{1'b0}}, ctz_clz};
		{8'bzz1zzzzz, ALUOP_CPOP   }: result = {{W_DATA-W_SHAMT-1{1'b0}}, cpop};
		{8'bzz1zzzzz, ALUOP_SEXT_B }: result = {{W_DATA-8{op_a[7]}}, op_a[7:0]};
		{8'bzz1zzzzz, ALUOP_SEXT_H }: result = {{W_DATA-16{op_a[15]}}, op_a[15:0]};
		{8'bzz1zzzzz, ALUOP_ZEXT_H }: result = {{W_DATA-16{1'b0}}, op_a[15:0]};
		{8'bzz1zzzzz, ALUOP_ORC_B  }: result = {{8{|op_a[31:24]}}, {8{|op_a[23:16]}}, {8{|op_a[15:8]}}, {8{|op_a[7:0]}}};
		{8'bzz1zzzzz, ALUOP_REV8   }: result = {op_a[7:0], op_a[15:8], op_a[23:16], op_a[31:24]};
		{8'bzz1zzzzz, ALUOP_ROL    }: result = shift_dout;
		{8'bzz1zzzzz, ALUOP_ROR    }: result = shift_dout;
		// Zbc
		{8'bzzz1zzzz, ALUOP_CLMUL  }: result = clmul;
		// Zbs
		{8'bzzzz1zzz, ALUOP_BCLR   }: result = op_a & ~zbs_mask;
		{8'bzzzz1zzz, ALUOP_BSET   }: result = op_a |  zbs_mask;
		{8'bzzzz1zzz, ALUOP_BINV   }: result = op_a ^  zbs_mask;
		{8'bzzzz1zzz, ALUOP_BEXT   }: result = {{W_DATA-1{1'b0}}, shift_dout[0]};
		// Zbkb
		{8'bzzzzz1zz, ALUOP_PACK   }: result = {op_b[15:0], op_a[15:0]};
		{8'bzzzzz1zz, ALUOP_PACKH  }: result = {{W_DATA-16{1'b0}}, op_b[7:0], op_a[7:0]};
		{8'bzzzzz1zz, ALUOP_BREV8  }: result = {op_a_rev[7:0], op_a_rev[15:8], op_a_rev[23:16], op_a_rev[31:24]};
		{8'bzzzzz1zz, ALUOP_UNZIP  }: result = unzip;
		{8'bzzzzz1zz, ALUOP_ZIP    }: result = zip;
		// Zbkx
		{8'bzzzzzz1z, ALUOP_XPERM  }: result = funct3_32b[2] ? xperm8 : xperm4;
		// Xh3bextm
		{8'bzzzzzzz1, ALUOP_BEXTM  }: result = shift_dout & {24'h0, {~(8'hfe << funct7_32b[3:1])}};

		default:                    result = bitwise;
	endcase
end

// ----------------------------------------------------------------------------
// Properties for base-ISA instructions

























endmodule


`default_nettype wire

/*****************************************************************************\
|                        Copyright (C) 2022 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

`default_nettype none

// The branch decision path through the ALU is slow because:
//
// - Sees immediates and PC on its inputs, as well as regs
// - Add/sub rather than just add (with complex decode of the sub condition)
// - 2 extra mux layers in front of adder if Zba extension is enabled
//
// So there is sometimes timing benefit to a dedicated branch comparator.

module hazard3_branchcmp #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
,
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// These really ought to be localparams, but are occasionally needed for
// passing flags around between modules, so are made available as parameters
// instead. It's ugly, but better scope hygiene than the preprocessor. These
// parameters should not be changed from their default values.

parameter W_REGADDR = 5,

parameter W_ALUOP   = 6,
parameter W_ALUSRC  = 1,
parameter W_MEMOP   = 5,
parameter W_BCOND   = 2,
parameter W_SHAMT   = 5,

parameter W_EXCEPT  = 4,
parameter W_MULOP   = 3
) (
	input  wire [31:0]        cir,
	input  wire [W_DATA-1:0]  op_a,
	input  wire [W_DATA-1:0]  op_b,
	output wire               cmp
);

/*****************************************************************************\
|                        Copyright (C) 2021 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// ALU operation selectors

localparam ALUOP_ADD     = 6'h00; 
localparam ALUOP_SUB     = 6'h01; 
localparam ALUOP_LT      = 6'h02;
localparam ALUOP_LTU     = 6'h04;
localparam ALUOP_AND     = 6'h06;
localparam ALUOP_OR      = 6'h07;
localparam ALUOP_XOR     = 6'h08;
localparam ALUOP_SRL     = 6'h09;
localparam ALUOP_SRA     = 6'h0a;
localparam ALUOP_SLL     = 6'h0b;
localparam ALUOP_MULDIV  = 6'h0c;
localparam ALUOP_RS2     = 6'h0d; // differs from AND/OR/XOR in [1:0]
// Bitmanip ALU operations (some also used by AMOs):
localparam ALUOP_SHXADD  = 6'h20;
localparam ALUOP_CLZ     = 6'h23;
localparam ALUOP_CPOP    = 6'h24;
localparam ALUOP_CTZ     = 6'h25;
localparam ALUOP_ANDN    = 6'h26; // Same LSBs as non-inverted
localparam ALUOP_ORN     = 6'h27; // Same LSBs as non-inverted
localparam ALUOP_XNOR    = 6'h28; // Same LSBs as non-inverted
localparam ALUOP_MAX     = 6'h29;
localparam ALUOP_MAXU    = 6'h2a;
localparam ALUOP_MIN     = 6'h2b;
localparam ALUOP_MINU    = 6'h2c;
localparam ALUOP_ORC_B   = 6'h2d;
localparam ALUOP_REV8    = 6'h2e;
localparam ALUOP_ROL     = 6'h2f;
localparam ALUOP_ROR     = 6'h30;
localparam ALUOP_SEXT_B  = 6'h31;
localparam ALUOP_SEXT_H  = 6'h32;
localparam ALUOP_ZEXT_H  = 6'h33;

localparam ALUOP_CLMUL   = 6'h34;

localparam ALUOP_BCLR    = 6'h35;
localparam ALUOP_BEXT    = 6'h36;
localparam ALUOP_BINV    = 6'h37;
localparam ALUOP_BSET    = 6'h38;

localparam ALUOP_PACK    = 6'h39;
localparam ALUOP_PACKH   = 6'h3a;
localparam ALUOP_BREV8   = 6'h3b;
localparam ALUOP_ZIP     = 6'h3c;
localparam ALUOP_UNZIP   = 6'h3d;

localparam ALUOP_BEXTM   = 6'h3e;

localparam ALUOP_XPERM   = 6'h3f;

// Parameters to control ALU input muxes. Bypass mux paths are
// controlled by X, so D has no parameters to choose these.

localparam ALUSRCA_RS1 = 1'h0;
localparam ALUSRCA_PC  = 1'h1;

localparam ALUSRCB_RS2 = 1'h0;
localparam ALUSRCB_IMM = 1'h1;

localparam MEMOP_LW   = 5'h00;
localparam MEMOP_LH   = 5'h01;
localparam MEMOP_LB   = 5'h02;
localparam MEMOP_LHU  = 5'h03;
localparam MEMOP_LBU  = 5'h04;
localparam MEMOP_SW   = 5'h05;
localparam MEMOP_SH   = 5'h06;
localparam MEMOP_SB   = 5'h07;

localparam MEMOP_LR_W = 5'h08;
localparam MEMOP_SC_W = 5'h09;
localparam MEMOP_AMO  = 5'h0a;
localparam MEMOP_NONE = 5'h10;

localparam BCOND_NEVER  = 2'h0;
localparam BCOND_ALWAYS = 2'h1;
localparam BCOND_ZERO   = 2'h2;
localparam BCOND_NZERO  = 2'h3;

// CSR access types

localparam CSR_WTYPE_W    = 2'h0;
localparam CSR_WTYPE_S    = 2'h1;
localparam CSR_WTYPE_C    = 2'h2;

// Exceptional condition signals which travel alongside (or instead of)
// instructions in the pipeline. These are speculative and can be flushed on
// e.g. branch mispredict. These mostly align with mcause values.

localparam EXCEPT_NONE           = 4'hf;

localparam EXCEPT_INSTR_MISALIGN = 4'h0;
localparam EXCEPT_INSTR_FAULT    = 4'h1;
localparam EXCEPT_INSTR_ILLEGAL  = 4'h2;
localparam EXCEPT_EBREAK         = 4'h3;
localparam EXCEPT_LOAD_ALIGN     = 4'h4;
localparam EXCEPT_LOAD_FAULT     = 4'h5;
localparam EXCEPT_STORE_ALIGN    = 4'h6;
localparam EXCEPT_STORE_FAULT    = 4'h7;
localparam EXCEPT_ECALL_U        = 4'h8;
// MRET, Return from M-mode: not really an exception, but handled like one
localparam EXCEPT_MRET           = 4'ha;
localparam EXCEPT_ECALL_M        = 4'hb;
// spare: c
// spare: d
// REFETCH: flush and refetch sequentially-following instructions, e.g. on
// executing fence.i. Jumps from stage 3 to get ordering against L/S dphase.
localparam EXCEPT_REFETCH        = 4'he;

// Operations for M extension (these are just instr[14:12])

localparam M_OP_MUL    = 3'h0;
localparam M_OP_MULH   = 3'h1;
localparam M_OP_MULHSU = 3'h2;
localparam M_OP_MULHU  = 3'h3;
localparam M_OP_DIV    = 3'h4;
localparam M_OP_DIVU   = 3'h5;
localparam M_OP_REM    = 3'h6;
localparam M_OP_REMU   = 3'h7;

wire [W_DATA-1:0] diff = op_a - op_b;

// funct3 instruction
// ------------------
// 000    BEQ
// 001    BNE
// 100    BLT
// 101    BGE
// 110    BLTU
// 111    BGEU

wire cmp_is_unsigned = cir[13];

wire lt = op_a[W_DATA-1] == op_b[W_DATA-1] ? diff[W_DATA-1] :
          cmp_is_unsigned                  ? op_b[W_DATA-1] :
                                             op_a[W_DATA-1] ;

assign cmp = cir[14] ? lt : op_a != op_b;

endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// MUL-only (cfg: MUL_FAST) and MUL/MULH/MULHU/MULHSU (cfg: MUL_FAST &&
// MULH_FAST) are handled by different circuits. In either case it's a simple
// behavioural multiply, and we rely on inference to get good performance on
// FPGA.

`default_nettype none

module hazard3_mul_fast #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
,
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// These really ought to be localparams, but are occasionally needed for
// passing flags around between modules, so are made available as parameters
// instead. It's ugly, but better scope hygiene than the preprocessor. These
// parameters should not be changed from their default values.

parameter W_REGADDR = 5,

parameter W_ALUOP   = 6,
parameter W_ALUSRC  = 1,
parameter W_MEMOP   = 5,
parameter W_BCOND   = 2,
parameter W_SHAMT   = 5,

parameter W_EXCEPT  = 4,
parameter W_MULOP   = 3
) (
	input  wire               clk,
	input  wire               rst_n,

	input  wire [W_MULOP-1:0] op,
	input  wire               op_vld,
	input  wire [W_DATA-1:0]  op_a,
	input  wire [W_DATA-1:0]  op_b,

	output wire [W_DATA-1:0] result,
	output reg               result_vld
);

/*****************************************************************************\
|                        Copyright (C) 2021 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// ALU operation selectors

localparam ALUOP_ADD     = 6'h00; 
localparam ALUOP_SUB     = 6'h01; 
localparam ALUOP_LT      = 6'h02;
localparam ALUOP_LTU     = 6'h04;
localparam ALUOP_AND     = 6'h06;
localparam ALUOP_OR      = 6'h07;
localparam ALUOP_XOR     = 6'h08;
localparam ALUOP_SRL     = 6'h09;
localparam ALUOP_SRA     = 6'h0a;
localparam ALUOP_SLL     = 6'h0b;
localparam ALUOP_MULDIV  = 6'h0c;
localparam ALUOP_RS2     = 6'h0d; // differs from AND/OR/XOR in [1:0]
// Bitmanip ALU operations (some also used by AMOs):
localparam ALUOP_SHXADD  = 6'h20;
localparam ALUOP_CLZ     = 6'h23;
localparam ALUOP_CPOP    = 6'h24;
localparam ALUOP_CTZ     = 6'h25;
localparam ALUOP_ANDN    = 6'h26; // Same LSBs as non-inverted
localparam ALUOP_ORN     = 6'h27; // Same LSBs as non-inverted
localparam ALUOP_XNOR    = 6'h28; // Same LSBs as non-inverted
localparam ALUOP_MAX     = 6'h29;
localparam ALUOP_MAXU    = 6'h2a;
localparam ALUOP_MIN     = 6'h2b;
localparam ALUOP_MINU    = 6'h2c;
localparam ALUOP_ORC_B   = 6'h2d;
localparam ALUOP_REV8    = 6'h2e;
localparam ALUOP_ROL     = 6'h2f;
localparam ALUOP_ROR     = 6'h30;
localparam ALUOP_SEXT_B  = 6'h31;
localparam ALUOP_SEXT_H  = 6'h32;
localparam ALUOP_ZEXT_H  = 6'h33;

localparam ALUOP_CLMUL   = 6'h34;

localparam ALUOP_BCLR    = 6'h35;
localparam ALUOP_BEXT    = 6'h36;
localparam ALUOP_BINV    = 6'h37;
localparam ALUOP_BSET    = 6'h38;

localparam ALUOP_PACK    = 6'h39;
localparam ALUOP_PACKH   = 6'h3a;
localparam ALUOP_BREV8   = 6'h3b;
localparam ALUOP_ZIP     = 6'h3c;
localparam ALUOP_UNZIP   = 6'h3d;

localparam ALUOP_BEXTM   = 6'h3e;

localparam ALUOP_XPERM   = 6'h3f;

// Parameters to control ALU input muxes. Bypass mux paths are
// controlled by X, so D has no parameters to choose these.

localparam ALUSRCA_RS1 = 1'h0;
localparam ALUSRCA_PC  = 1'h1;

localparam ALUSRCB_RS2 = 1'h0;
localparam ALUSRCB_IMM = 1'h1;

localparam MEMOP_LW   = 5'h00;
localparam MEMOP_LH   = 5'h01;
localparam MEMOP_LB   = 5'h02;
localparam MEMOP_LHU  = 5'h03;
localparam MEMOP_LBU  = 5'h04;
localparam MEMOP_SW   = 5'h05;
localparam MEMOP_SH   = 5'h06;
localparam MEMOP_SB   = 5'h07;

localparam MEMOP_LR_W = 5'h08;
localparam MEMOP_SC_W = 5'h09;
localparam MEMOP_AMO  = 5'h0a;
localparam MEMOP_NONE = 5'h10;

localparam BCOND_NEVER  = 2'h0;
localparam BCOND_ALWAYS = 2'h1;
localparam BCOND_ZERO   = 2'h2;
localparam BCOND_NZERO  = 2'h3;

// CSR access types

localparam CSR_WTYPE_W    = 2'h0;
localparam CSR_WTYPE_S    = 2'h1;
localparam CSR_WTYPE_C    = 2'h2;

// Exceptional condition signals which travel alongside (or instead of)
// instructions in the pipeline. These are speculative and can be flushed on
// e.g. branch mispredict. These mostly align with mcause values.

localparam EXCEPT_NONE           = 4'hf;

localparam EXCEPT_INSTR_MISALIGN = 4'h0;
localparam EXCEPT_INSTR_FAULT    = 4'h1;
localparam EXCEPT_INSTR_ILLEGAL  = 4'h2;
localparam EXCEPT_EBREAK         = 4'h3;
localparam EXCEPT_LOAD_ALIGN     = 4'h4;
localparam EXCEPT_LOAD_FAULT     = 4'h5;
localparam EXCEPT_STORE_ALIGN    = 4'h6;
localparam EXCEPT_STORE_FAULT    = 4'h7;
localparam EXCEPT_ECALL_U        = 4'h8;
// MRET, Return from M-mode: not really an exception, but handled like one
localparam EXCEPT_MRET           = 4'ha;
localparam EXCEPT_ECALL_M        = 4'hb;
// spare: c
// spare: d
// REFETCH: flush and refetch sequentially-following instructions, e.g. on
// executing fence.i. Jumps from stage 3 to get ordering against L/S dphase.
localparam EXCEPT_REFETCH        = 4'he;

// Operations for M extension (these are just instr[14:12])

localparam M_OP_MUL    = 3'h0;
localparam M_OP_MULH   = 3'h1;
localparam M_OP_MULHSU = 3'h2;
localparam M_OP_MULHU  = 3'h3;
localparam M_OP_DIV    = 3'h4;
localparam M_OP_DIVU   = 3'h5;
localparam M_OP_REM    = 3'h6;
localparam M_OP_REMU   = 3'h7;

localparam XLEN = W_DATA;

//synthesis translate_off
generate if (MULH_FAST && !MUL_FAST) begin: err_require_mul_fast_for_mulh
	initial $fatal("%m: MULH_FAST requires that MUL_FAST is also set.");
end endgenerate
generate if (MUL_FASTER && !MUL_FAST) begin: err_require_mul_fast_for_faster
	initial $fatal("%m: MUL_FASTER requires that MUL_FAST is also set.");
end endgenerate
//synthesis translate_on

// Latency of 1:
always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		result_vld <= 1'b0;
	end else begin
		result_vld <= op_vld;
	end
end

// ----------------------------------------------------------------------------
// Fast MUL only

generate
if (!MULH_FAST) begin: mul_only

// This pipestage is folded into the front of the DSP tiles on UP5k. Note the
// intention is to register the bypassed core regs at the end of X (since
// bypass is quite slow), then perform multiply combinatorially in stage M,
// and mux into MW result register.

reg [XLEN-1:0] op_a_r;
reg [XLEN-1:0] op_b_r;

if (MUL_FASTER) begin: op_passthrough
	always @ (*) begin
		op_a_r = op_a;
		op_b_r = op_b;
	end
end else begin: op_register
	always @ (posedge clk) begin
		if (op_vld) begin
			op_a_r <= op_a;
			op_b_r <= op_b;
		end
	end
end

// This should be inferred as 3 DSP tiles on UP5k:
//
// 1. Register then multiply a[15: 0] and b[15: 0]
// 2. Register then multiply a[31:16] and b[15: 0], then directly add output of 1
// 3. Register then multiply a[15: 0] and b[31:16], then directly add output of 2
//
// So there is quite a long path (1x 16-bit multiply, then 2x 16-bit add). On
// other platforms you may just end up with a pile of gates.



assign result = op_a_r * op_b_r;









// ----------------------------------------------------------------------------
// Fast MUL/MULH/MULHU/MULHSU

end else begin: mul_and_mulh

reg [XLEN-1:0]    op_a_r;
reg [XLEN-1:0]    op_b_r;
reg [W_MULOP-1:0] op_r;

if (MUL_FASTER) begin: op_passthrough
	always @ (*) begin
		op_a_r = op_a;
		op_b_r = op_b;
		op_r = op;
	end
end else begin: op_register
	always @ (posedge clk) begin
		if (op_vld) begin
			op_a_r <= op_a;
			op_b_r <= op_b;
			op_r <= op;
		end
	end
end

wire op_a_signed = op_r == M_OP_MULH || op_r == M_OP_MULHSU;
wire op_b_signed = op_r == M_OP_MULH;

wire [2*XLEN-1:0] op_a_sext = {
	{XLEN{op_a_r[XLEN - 1] && op_a_signed}},
	op_a_r
};

wire [2*XLEN-1:0] op_b_sext = {
	{XLEN{op_b_r[XLEN - 1] && op_b_signed}},
	op_b_r
};

wire [2*XLEN-1:0] result_full = op_a_sext * op_b_sext;



assign result = op_r == M_OP_MUL ? result_full[0 +: XLEN] : result_full[XLEN +: XLEN];











end
endgenerate

endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Combined multiply/divide/modulo circuit. All operations performed at 1 bit
// per clock; aiming for minimal resource usage on iCE40 FPGA. Optionally the
// circuit can be unrolled for slightly higher performance.
//
// When op_kill is high, the current calculation halts immediately. op_vld can
// be asserted on the same cycle, and the new calculation begins without
// delay, regardless of op_rdy. This may be used by the processor on e.g.
// mispredict or trap.
//
// The actual multiply/divide hardware is unsigned. We handle signedness at
// input/output.

`default_nettype none

module hazard3_muldiv_seq #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
,
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// These really ought to be localparams, but are occasionally needed for
// passing flags around between modules, so are made available as parameters
// instead. It's ugly, but better scope hygiene than the preprocessor. These
// parameters should not be changed from their default values.

parameter W_REGADDR = 5,

parameter W_ALUOP   = 6,
parameter W_ALUSRC  = 1,
parameter W_MEMOP   = 5,
parameter W_BCOND   = 2,
parameter W_SHAMT   = 5,

parameter W_EXCEPT  = 4,
parameter W_MULOP   = 3
) (
	input  wire               clk,
	input  wire               rst_n,
	input  wire [W_MULOP-1:0] op,
	input  wire               op_vld,
	output wire               op_rdy,
	input  wire               op_kill,
	input  wire [W_DATA-1:0]  op_a,
	input  wire [W_DATA-1:0]  op_b,

	output wire [W_DATA-1:0]  result_h, // mulh* or rem*
	output wire [W_DATA-1:0]  result_l, // mul   or div*
	output wire               result_vld
);

/*****************************************************************************\
|                        Copyright (C) 2021 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// ALU operation selectors

localparam ALUOP_ADD     = 6'h00; 
localparam ALUOP_SUB     = 6'h01; 
localparam ALUOP_LT      = 6'h02;
localparam ALUOP_LTU     = 6'h04;
localparam ALUOP_AND     = 6'h06;
localparam ALUOP_OR      = 6'h07;
localparam ALUOP_XOR     = 6'h08;
localparam ALUOP_SRL     = 6'h09;
localparam ALUOP_SRA     = 6'h0a;
localparam ALUOP_SLL     = 6'h0b;
localparam ALUOP_MULDIV  = 6'h0c;
localparam ALUOP_RS2     = 6'h0d; // differs from AND/OR/XOR in [1:0]
// Bitmanip ALU operations (some also used by AMOs):
localparam ALUOP_SHXADD  = 6'h20;
localparam ALUOP_CLZ     = 6'h23;
localparam ALUOP_CPOP    = 6'h24;
localparam ALUOP_CTZ     = 6'h25;
localparam ALUOP_ANDN    = 6'h26; // Same LSBs as non-inverted
localparam ALUOP_ORN     = 6'h27; // Same LSBs as non-inverted
localparam ALUOP_XNOR    = 6'h28; // Same LSBs as non-inverted
localparam ALUOP_MAX     = 6'h29;
localparam ALUOP_MAXU    = 6'h2a;
localparam ALUOP_MIN     = 6'h2b;
localparam ALUOP_MINU    = 6'h2c;
localparam ALUOP_ORC_B   = 6'h2d;
localparam ALUOP_REV8    = 6'h2e;
localparam ALUOP_ROL     = 6'h2f;
localparam ALUOP_ROR     = 6'h30;
localparam ALUOP_SEXT_B  = 6'h31;
localparam ALUOP_SEXT_H  = 6'h32;
localparam ALUOP_ZEXT_H  = 6'h33;

localparam ALUOP_CLMUL   = 6'h34;

localparam ALUOP_BCLR    = 6'h35;
localparam ALUOP_BEXT    = 6'h36;
localparam ALUOP_BINV    = 6'h37;
localparam ALUOP_BSET    = 6'h38;

localparam ALUOP_PACK    = 6'h39;
localparam ALUOP_PACKH   = 6'h3a;
localparam ALUOP_BREV8   = 6'h3b;
localparam ALUOP_ZIP     = 6'h3c;
localparam ALUOP_UNZIP   = 6'h3d;

localparam ALUOP_BEXTM   = 6'h3e;

localparam ALUOP_XPERM   = 6'h3f;

// Parameters to control ALU input muxes. Bypass mux paths are
// controlled by X, so D has no parameters to choose these.

localparam ALUSRCA_RS1 = 1'h0;
localparam ALUSRCA_PC  = 1'h1;

localparam ALUSRCB_RS2 = 1'h0;
localparam ALUSRCB_IMM = 1'h1;

localparam MEMOP_LW   = 5'h00;
localparam MEMOP_LH   = 5'h01;
localparam MEMOP_LB   = 5'h02;
localparam MEMOP_LHU  = 5'h03;
localparam MEMOP_LBU  = 5'h04;
localparam MEMOP_SW   = 5'h05;
localparam MEMOP_SH   = 5'h06;
localparam MEMOP_SB   = 5'h07;

localparam MEMOP_LR_W = 5'h08;
localparam MEMOP_SC_W = 5'h09;
localparam MEMOP_AMO  = 5'h0a;
localparam MEMOP_NONE = 5'h10;

localparam BCOND_NEVER  = 2'h0;
localparam BCOND_ALWAYS = 2'h1;
localparam BCOND_ZERO   = 2'h2;
localparam BCOND_NZERO  = 2'h3;

// CSR access types

localparam CSR_WTYPE_W    = 2'h0;
localparam CSR_WTYPE_S    = 2'h1;
localparam CSR_WTYPE_C    = 2'h2;

// Exceptional condition signals which travel alongside (or instead of)
// instructions in the pipeline. These are speculative and can be flushed on
// e.g. branch mispredict. These mostly align with mcause values.

localparam EXCEPT_NONE           = 4'hf;

localparam EXCEPT_INSTR_MISALIGN = 4'h0;
localparam EXCEPT_INSTR_FAULT    = 4'h1;
localparam EXCEPT_INSTR_ILLEGAL  = 4'h2;
localparam EXCEPT_EBREAK         = 4'h3;
localparam EXCEPT_LOAD_ALIGN     = 4'h4;
localparam EXCEPT_LOAD_FAULT     = 4'h5;
localparam EXCEPT_STORE_ALIGN    = 4'h6;
localparam EXCEPT_STORE_FAULT    = 4'h7;
localparam EXCEPT_ECALL_U        = 4'h8;
// MRET, Return from M-mode: not really an exception, but handled like one
localparam EXCEPT_MRET           = 4'ha;
localparam EXCEPT_ECALL_M        = 4'hb;
// spare: c
// spare: d
// REFETCH: flush and refetch sequentially-following instructions, e.g. on
// executing fence.i. Jumps from stage 3 to get ordering against L/S dphase.
localparam EXCEPT_REFETCH        = 4'he;

// Operations for M extension (these are just instr[14:12])

localparam M_OP_MUL    = 3'h0;
localparam M_OP_MULH   = 3'h1;
localparam M_OP_MULHSU = 3'h2;
localparam M_OP_MULHU  = 3'h3;
localparam M_OP_DIV    = 3'h4;
localparam M_OP_DIVU   = 3'h5;
localparam M_OP_REM    = 3'h6;
localparam M_OP_REMU   = 3'h7;

//synthesis translate_off
generate if (|(MULDIV_UNROLL & (MULDIV_UNROLL - 1)) || ~|MULDIV_UNROLL) begin: err_pow2
	initial $fatal("%m: MULDIV_UNROLL must be a positive power of 2");
end endgenerate
//synthesis translate_on

localparam XLEN = W_DATA;
parameter W_CTR = $clog2(XLEN + 1);

// ----------------------------------------------------------------------------
// Operation decode, operand sign adjustment

// On the first cycle, op_a and op_b go straight through to the accumulator
// and the divisor/multiplicand register. They are then adjusted in-place
// on the next cycle. This allows the same circuits to be reused for sign
// adjustment before output (and helps input timing).

reg [W_MULOP-1:0] op_r;
reg [2*XLEN-1:0]  accum;
reg [XLEN-1:0]    op_b_r;
reg               op_a_neg_r;
reg               op_b_neg_r;

wire op_a_signed =
	op_r == M_OP_MULH ||
	op_r == M_OP_MULHSU ||
	op_r == M_OP_DIV ||
	op_r == M_OP_REM;

wire op_b_signed =
	op_r == M_OP_MULH ||
	op_r == M_OP_DIV ||
	op_r == M_OP_REM;

wire op_a_neg = op_a_signed && accum[XLEN-1];
wire op_b_neg = op_b_signed && op_b_r[XLEN-1];

// Non-divide parts of the circuit should be constant-folded if all the MUL
// operations are handled by the fast multiplier

wire is_div = op_r[2] || (MUL_FAST && MULH_FAST);

// Controls for modifying sign of all/part of accumulator
wire accum_neg_l;
wire accum_inv_h;
wire accum_incr_h;

// ----------------------------------------------------------------------------
// Arithmetic circuit

// Combinatorials:
reg [2*XLEN-1:0] accum_next;
reg [2*XLEN-1:0] addend;
reg [2*XLEN-1:0] shift_tmp;
reg [2*XLEN-1:0] addsub_tmp;
reg              neg_l_borrow;

always @ (*) begin: alu
	integer i;
	// Multiply/divide iteration layers
	accum_next = accum;
	addend = {2*XLEN{1'b0}};
	addsub_tmp = {2*XLEN{1'b0}};
	neg_l_borrow = 1'b0;
	for (i = 0; i < MULDIV_UNROLL; i = i + 1) begin
		addend = {is_div && |op_b_r, op_b_r, {XLEN-1{1'b0}}};
		shift_tmp = is_div ? accum_next : accum_next >> 1;
		addsub_tmp = shift_tmp + addend;
		accum_next = (is_div ? !addsub_tmp[2 * XLEN - 1] : accum_next[0]) ?
			addsub_tmp : shift_tmp;
		if (is_div)
			accum_next = {accum_next[2*XLEN-2:0], !addsub_tmp[2 * XLEN - 1]};
	end
	// Alternative path for negation of all/part of accumulator
	if (accum_neg_l)
		{neg_l_borrow, accum_next[XLEN-1:0]} = {~accum[XLEN-1:0]} + 1'b1;
	if (accum_incr_h || accum_inv_h)
		accum_next[XLEN +: XLEN] = (accum[XLEN +: XLEN] ^ {XLEN{accum_inv_h}})
			+ {{XLEN-1{1'b0}}, accum_incr_h};
end

// ----------------------------------------------------------------------------
// Main state machine

reg sign_preadj_done;
reg [W_CTR-1:0] ctr;
reg sign_postadj_done;
reg sign_postadj_carry;

localparam CTR_TOP = XLEN[W_CTR-1:0];

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		ctr <= {W_CTR{1'b0}};
		sign_preadj_done <= 1'b1;
		sign_postadj_done <= 1'b1;
		sign_postadj_carry <= 1'b0;
		op_r <= {W_MULOP{1'b0}};
		op_a_neg_r <= 1'b0;
		op_b_neg_r <= 1'b0;
		op_b_r <= {XLEN{1'b0}};
		accum <= {XLEN*2{1'b0}};
	end else if (op_kill || (op_vld && op_rdy)) begin
		// Initialise circuit with operands + state
		ctr <= op_vld ? CTR_TOP : {W_CTR{1'b0}};
		sign_preadj_done <= !op_vld;
		sign_postadj_done <= !op_vld;
		sign_postadj_carry <= 1'b0;
		op_r <= op;
		op_b_r <= op_b;
		accum <= {{XLEN{1'b0}}, op_a};
	end else if (!sign_preadj_done) begin
		// Pre-adjust sign if necessary, else perform first iteration immediately
		op_a_neg_r <= op_a_neg;
		op_b_neg_r <= op_b_neg;
		sign_preadj_done <= 1'b1;
		if (accum_neg_l || (op_b_neg ^ is_div)) begin
			if (accum_neg_l)
				accum[0 +: XLEN] <= accum_next[0 +: XLEN];
			if (op_b_neg ^ is_div)
				op_b_r <= -op_b_r;
		end else begin
			ctr <= ctr - MULDIV_UNROLL[W_CTR-1:0];
			accum <= accum_next;
		end
	end else if (|ctr) begin
		ctr <= ctr - MULDIV_UNROLL[W_CTR-1:0];
		accum <= accum_next;
	end else if (!sign_postadj_done || sign_postadj_carry) begin
		sign_postadj_done <= 1'b1;
		if (accum_inv_h || accum_incr_h)
			accum[XLEN +: XLEN] <= accum_next[XLEN +: XLEN];
		if (accum_neg_l) begin
			accum[0 +: XLEN] <= accum_next[0 +: XLEN];
			if (!is_div) begin
				sign_postadj_carry <= neg_l_borrow;
				sign_postadj_done <= !neg_l_borrow;
			end
		end
	end
end

// ----------------------------------------------------------------------------
// Sign adjustment control

// Pre-adjustment: for any a, b we want |a|, |b|. Note that the magnitude of any
// 32-bit signed integer is representable by a 32-bit unsigned integer.

// Post-adjustment for division:
// We seek q, r to satisfy a = b * q + r, where a and b are given,
// and |r| < |b|. One way to do this is if
// sgn(r) = sgn(a)
// sgn(q) = sgn(a) ^ sgn(b)
// This has additional nice properties like
// -(a / b) = (-a) / b = a / (-b)

// Post-adjustment for multiplication:
// We have calculated the 2*XLEN result of |a| * |b|.
// Negate the entire accumulator if sgn(a) ^ sgn(b).
// This is done in two steps (to share div/mod circuit, and avoid 64-bit carry):
// - Negate lower half of accumulator, and invert upper half
// - Increment upper half if lower half carried

wire do_postadj = ~|{ctr, sign_postadj_done};
wire op_signs_differ = op_a_neg_r ^ op_b_neg_r;

assign accum_neg_l =
	!sign_preadj_done && op_a_neg ||
	do_postadj && !sign_postadj_carry && op_signs_differ && !(is_div && ~|op_b_r);

assign {accum_incr_h, accum_inv_h} =
	do_postadj &&  is_div && op_a_neg_r                             ? 2'b11 :
	do_postadj && !is_div && op_signs_differ && !sign_postadj_carry ? 2'b01 :
	do_postadj && !is_div && op_signs_differ &&  sign_postadj_carry ? 2'b10 :
	                                                                  2'b00 ;

// ----------------------------------------------------------------------------
// Outputs

assign op_rdy = ~|{ctr, accum_neg_l, accum_incr_h, accum_inv_h};
assign result_vld = op_rdy;



assign {result_h, result_l} = accum;

































// ----------------------------------------------------------------------------
// Interface properties





























endmodule


`default_nettype wire

/*****************************************************************************\
|                         Copyright (C) 2022 Luke Wren                        |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// req: one-hot bitmap
// idx: index of the sole set bit in req

`default_nettype none

module hazard3_onehot_encode #(
	parameter W_REQ = 16,
	parameter W_GNT = $clog2(W_REQ) // do not modify
) (
	input  wire [W_REQ-1:0] req,
	output reg  [W_GNT-1:0] gnt
);

always @ (*) begin: encode
	reg [W_GNT:0] i;
	gnt = {W_GNT{1'b0}};
	for (i = 0; i < W_REQ; i = i + 1) begin
		gnt = gnt | ({W_GNT{req[i[W_GNT-1:0]]}} & i[W_GNT-1:0]);
	end
end

endmodule


`default_nettype wire

/*****************************************************************************\
|                         Copyright (C) 2022 Luke Wren                        |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// req: bitmap
// idx: bitmap with all bits clear except the least- (HIGHEST_WINS=0) or
//      most- (HIGHEST_WINS=1) significant set bit in req.

`default_nettype none

module hazard3_onehot_priority #(
	parameter W_REQ        = 16,
	parameter HIGHEST_WINS = 0
) (
	input  wire [W_REQ-1:0] req,
	output reg  [W_REQ-1:0] gnt
);

always @ (*) begin: select
	integer i;
	for (i = 0; i < W_REQ; i = i + 1) begin
		gnt[i] = req[i] && ~|(req & (
			HIGHEST_WINS ? ~({W_REQ{1'b1}} >> (W_REQ - 1 - i)) : ~({W_REQ{1'b1}} << i)
		));
	end
end

endmodule


`default_nettype wire

/*****************************************************************************\
|                         Copyright (C) 2022 Luke Wren                        |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// req:      bitmap of requests
// priority: packed array of dynamic priority level of each request
// gnt:      one-hot bitmap with the highest-priority request.

`default_nettype none

module hazard3_onehot_priority_dynamic #(
	parameter W_REQ                 = 8,
	parameter N_PRIORITIES          = 2,
	parameter PRIORITY_HIGHEST_WINS = 1, // If 1, numerically highest level has greatest priority.
	                                     // Otherwise, numerically lowest wins.
	parameter TIEBREAK_HIGHEST_WINS = 0, // If 1, highest-numbered request at the highest priority
	                                     // level wins the tiebreak. Otherwise, lowest-numbered.
	// Do not modify:
	parameter W_PRIORITY            = $clog2(N_PRIORITIES)
) (
	input  wire [W_REQ*W_PRIORITY-1:0] pri,
	input  wire [W_REQ-1:0]            req,
	output wire [W_REQ-1:0]            gnt
);

// 1. Stratify requests according to their level
reg [W_REQ-1:0]        req_stratified [0:N_PRIORITIES-1];
reg [N_PRIORITIES-1:0] level_has_req;

always @ (*) begin: stratify
	reg signed [31:0] i, j;
	for (i = 0; i < N_PRIORITIES; i = i + 1) begin
		for (j = 0; j < W_REQ; j = j + 1) begin
			req_stratified[i][j] = req[j] &&
				pri[W_PRIORITY * j +: W_PRIORITY] == i[W_PRIORITY-1:0];
		end
		level_has_req[i] = |req_stratified[i];
	end
end

// 2. Select the highest level with active requests
wire [N_PRIORITIES-1:0] active_layer_sel;

hazard3_onehot_priority #(
	.W_REQ        (N_PRIORITIES),
	.HIGHEST_WINS (PRIORITY_HIGHEST_WINS)
) prisel_layer (
	.req (level_has_req),
	.gnt (active_layer_sel)
);

// 3. Mask only those requests at this level
reg [W_REQ-1:0] reqs_from_highest_layer;

always @ (*) begin: mux_reqs_by_layer
	integer i;
	reqs_from_highest_layer = {W_REQ{1'b0}};
	for (i = 0; i < N_PRIORITIES; i = i + 1)
		reqs_from_highest_layer = reqs_from_highest_layer |
			(req_stratified[i] & {W_REQ{active_layer_sel[i]}});
end

// 4. Do a standard priority select on those requests as a tie break
hazard3_onehot_priority #(
	.W_REQ        (W_REQ),
	.HIGHEST_WINS (TIEBREAK_HIGHEST_WINS)
) prisel_tiebreak (
	.req (reqs_from_highest_layer),
	.gnt (gnt)
);

endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// req: bitmap
// gnt: index of least set bit (HIGHEST_WINS=0) or most set bit (HIGHEST_WINS=1)

`default_nettype none

module hazard3_priority_encode #(
	parameter W_REQ        = 16,
	parameter HIGHEST_WINS = 0,
	parameter W_GNT        = $clog2(W_REQ) // do not modify
) (
	input  wire [W_REQ-1:0] req,
	output wire [W_GNT-1:0] gnt
);

wire [W_REQ-1:0] gnt_onehot;

hazard3_onehot_priority #(
	.W_REQ        (W_REQ),
	.HIGHEST_WINS (HIGHEST_WINS)
) priority_u (
	.req (req),
	.gnt (gnt_onehot)
);

hazard3_onehot_encode #(
	.W_REQ (W_REQ)
) encode_u (
	.req (gnt_onehot),
	.gnt (gnt)
);

endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Implement the three shifts (left logical, right logical, right arithmetic)
// using a single log-type barrel shifter. Around 240 LUTs for 32 bits.
// (7 layers of 32 2-input muxes, some extra LUTs and LUT inputs used for arith)

`default_nettype none

module hazard3_shift_barrel #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
,
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// These really ought to be localparams, but are occasionally needed for
// passing flags around between modules, so are made available as parameters
// instead. It's ugly, but better scope hygiene than the preprocessor. These
// parameters should not be changed from their default values.

parameter W_REGADDR = 5,

parameter W_ALUOP   = 6,
parameter W_ALUSRC  = 1,
parameter W_MEMOP   = 5,
parameter W_BCOND   = 2,
parameter W_SHAMT   = 5,

parameter W_EXCEPT  = 4,
parameter W_MULOP   = 3
) (
	input wire [W_DATA-1:0]  din,
	input wire [W_SHAMT-1:0] shamt,
	input wire               right_nleft,
	input wire               rotate,
	input wire               arith,
	output reg [W_DATA-1:0]  dout
);

reg [W_DATA-1:0] din_rev;
reg [W_DATA-1:0] shift_accum;
reg sext; // haha

always @ (*) begin: shift
	integer i;

	for (i = 0; i < W_DATA; i = i + 1)
		din_rev[i] = right_nleft ? din[W_DATA - 1 - i] : din[i];

	sext = arith && din_rev[0];

	shift_accum = din_rev;
	for (i = 0; i < W_SHAMT; i = i + 1) begin
		if (shamt[i]) begin
			shift_accum = (shift_accum << (1 << i)) |
				({W_DATA{sext}} & ~({W_DATA{1'b1}} << (1 << i))) |
				({W_DATA{rotate && |EXTENSION_ZBB}} & (shift_accum >> (W_DATA - (1 << i))));
		end
	end

	for (i = 0; i < W_DATA; i = i + 1)
		dout[i] = right_nleft ? shift_accum[W_DATA - 1 - i] : shift_accum[i];
end













endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

`default_nettype none

// Control and Status Registers (CSRs)
// Also includes CSR-related logic like interrupt enable/masking,
// trap vector calculation.

module hazard3_csr #(
	parameter XLEN            = 32,   // Must be 32
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
,
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// These really ought to be localparams, but are occasionally needed for
// passing flags around between modules, so are made available as parameters
// instead. It's ugly, but better scope hygiene than the preprocessor. These
// parameters should not be changed from their default values.

parameter W_REGADDR = 5,

parameter W_ALUOP   = 6,
parameter W_ALUSRC  = 1,
parameter W_MEMOP   = 5,
parameter W_BCOND   = 2,
parameter W_SHAMT   = 5,

parameter W_EXCEPT  = 4,
parameter W_MULOP   = 3
) (
	input  wire               clk,
	input  wire               clk_always_on,
	input  wire               rst_n,

	// Debug signalling
	output reg                debug_mode,

	input  wire               dbg_req_halt,
	input  wire               dbg_req_halt_on_reset,
	input  wire               dbg_req_resume,

	output wire               dbg_instr_caught_exception,
	output wire               dbg_instr_caught_ebreak,

	input  wire [W_DATA-1:0]  dbg_data0_rdata,
	output wire [W_DATA-1:0]  dbg_data0_wdata,
	output wire               dbg_data0_wen,

	output wire [W_ADDR-1:0]  debug_dpc_wdata,
	output wire               debug_dpc_wen,
	input  wire [W_ADDR-1:0]  debug_dpc_rdata,

	// Read port is combinatorial.
	// Write port is synchronous, and write effects will be observed on the next clock cycle.
	// The *_soon strobes are versions which the core does not gate with its stall signal.
	// These are needed because:
	// - Core stall is a function of bus stall
	// - Illegal CSR accesses produce trap entry
	// - Trap entry (not necessarily caused by CSR access) gates outgoing bus accesses
	// - Through-paths from e.g. hready to htrans are problematic for timing/implementation
	input  wire [11:0]        addr,
	input  wire [XLEN-1:0]    wdata,
	input  wire               wen,
	input  wire               wen_soon, // wen will be asserted once some stall condition clears
	input  wire               wen_raw,  // raw, ungated instruction decode signal, for D-mode regs
	input  wire [1:0]         wtype,
	output reg  [XLEN-1:0]    rdata,
	input  wire               ren,
	input  wire               ren_soon, // ren will be asserted once some stall condition clears
	output wire               illegal,
	output wire               write_is_fetch_ordered,

	// Trap signalling
	// *We* tell the core that we are taking a trap, and where to, based on:
	// - Synchronous exception inputs from the core
	// - External IRQ signals
	// - Masking etc based on the state of CSRs like mie
	//
	// We do this by raising trap_enter_vld, and keeping it raised until trap_enter_rdy
	// goes high. trap_addr has the absolute value of trap target address.
	// Once trap_enter_vld && _rdy, mepc_in is copied to mepc, and other trap state is set.
	//
	// Note that an exception input can go away, e.g. if the pipe gets flushed. In this
	// case we lower trap_enter_vld.
	output wire [XLEN-1:0]     trap_addr,
	output wire                trap_is_irq,
	output wire                trap_enter_vld,
	input  wire                trap_enter_rdy,
	// True when we are about to trap, but are waiting for an excepting or
	// potentially-excepting instruction to clear M first. The instruction in X
	// is suppressed, X PC does not increment but still tracks exception
	// addresses.
	output wire                trap_enter_soon,
	// We need to know about load/stores in data phase because their exception
	// status is still unknown, so fence them off before entering debug mode.
	input  wire                loadstore_dphase_pending,
	// Asserted when the instruction in stage 2 can't be squashed (e.g. to
	// keep an AHB address phase stable), or instruction fetch request can't
	// be issued (e.g. waiting for wake from wfi state).
	input  wire                delay_irq_entry,
	input  wire [XLEN-1:0]     mepc_in,

	// Power control signalling
	output wire                pwr_allow_clkgate,
	output wire                pwr_allow_power_down,
	output wire                pwr_allow_sleep_on_block,
	output wire                pwr_wfi_wakeup_req,

	// Each of these may be performed at a different privilege level from the others:
	output wire                m_mode_execution,
	output wire                m_mode_trap_entry,
	output wire                m_mode_loadstore,

	// Exceptions must *not* be a function of bus stall.
	input  wire [W_EXCEPT-1:0] except,
	input  wire                except_to_d_mode,

	// Level-sensitive interrupt sources
	input  wire [NUM_IRQS-1:0] irq,
	input  wire                irq_software,
	input  wire                irq_timer,

	// PMP config interface
	output wire [11:0]         pmp_cfg_addr,
	output reg                 pmp_cfg_wen,
	output wire [W_DATA-1:0]   pmp_cfg_wdata,
	input  wire [W_DATA-1:0]   pmp_cfg_rdata,

	// Trigger config interface
	output wire [11:0]         trig_cfg_addr,
	output reg                 trig_cfg_wen,
	output wire [W_DATA-1:0]   trig_cfg_wdata,
	input  wire [W_DATA-1:0]   trig_cfg_rdata,
	output wire                trig_m_en,

	// Interrupt/exception trigger events
	output wire                trig_event_interrupt,
	output wire                trig_event_exception,
	output wire [3:0]          trig_event_trap_cause,

	// Other CSR-specific signalling
	output wire                clear_excl_on_trap_exit,
	output wire                trap_wfi,
	input  wire                instr_ret,
	input  wire [W_DATA-1:0]   mhartid_val,
	input  wire [3:0]          eco_version
);

/*****************************************************************************\
|                        Copyright (C) 2021 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// ALU operation selectors

localparam ALUOP_ADD     = 6'h00; 
localparam ALUOP_SUB     = 6'h01; 
localparam ALUOP_LT      = 6'h02;
localparam ALUOP_LTU     = 6'h04;
localparam ALUOP_AND     = 6'h06;
localparam ALUOP_OR      = 6'h07;
localparam ALUOP_XOR     = 6'h08;
localparam ALUOP_SRL     = 6'h09;
localparam ALUOP_SRA     = 6'h0a;
localparam ALUOP_SLL     = 6'h0b;
localparam ALUOP_MULDIV  = 6'h0c;
localparam ALUOP_RS2     = 6'h0d; // differs from AND/OR/XOR in [1:0]
// Bitmanip ALU operations (some also used by AMOs):
localparam ALUOP_SHXADD  = 6'h20;
localparam ALUOP_CLZ     = 6'h23;
localparam ALUOP_CPOP    = 6'h24;
localparam ALUOP_CTZ     = 6'h25;
localparam ALUOP_ANDN    = 6'h26; // Same LSBs as non-inverted
localparam ALUOP_ORN     = 6'h27; // Same LSBs as non-inverted
localparam ALUOP_XNOR    = 6'h28; // Same LSBs as non-inverted
localparam ALUOP_MAX     = 6'h29;
localparam ALUOP_MAXU    = 6'h2a;
localparam ALUOP_MIN     = 6'h2b;
localparam ALUOP_MINU    = 6'h2c;
localparam ALUOP_ORC_B   = 6'h2d;
localparam ALUOP_REV8    = 6'h2e;
localparam ALUOP_ROL     = 6'h2f;
localparam ALUOP_ROR     = 6'h30;
localparam ALUOP_SEXT_B  = 6'h31;
localparam ALUOP_SEXT_H  = 6'h32;
localparam ALUOP_ZEXT_H  = 6'h33;

localparam ALUOP_CLMUL   = 6'h34;

localparam ALUOP_BCLR    = 6'h35;
localparam ALUOP_BEXT    = 6'h36;
localparam ALUOP_BINV    = 6'h37;
localparam ALUOP_BSET    = 6'h38;

localparam ALUOP_PACK    = 6'h39;
localparam ALUOP_PACKH   = 6'h3a;
localparam ALUOP_BREV8   = 6'h3b;
localparam ALUOP_ZIP     = 6'h3c;
localparam ALUOP_UNZIP   = 6'h3d;

localparam ALUOP_BEXTM   = 6'h3e;

localparam ALUOP_XPERM   = 6'h3f;

// Parameters to control ALU input muxes. Bypass mux paths are
// controlled by X, so D has no parameters to choose these.

localparam ALUSRCA_RS1 = 1'h0;
localparam ALUSRCA_PC  = 1'h1;

localparam ALUSRCB_RS2 = 1'h0;
localparam ALUSRCB_IMM = 1'h1;

localparam MEMOP_LW   = 5'h00;
localparam MEMOP_LH   = 5'h01;
localparam MEMOP_LB   = 5'h02;
localparam MEMOP_LHU  = 5'h03;
localparam MEMOP_LBU  = 5'h04;
localparam MEMOP_SW   = 5'h05;
localparam MEMOP_SH   = 5'h06;
localparam MEMOP_SB   = 5'h07;

localparam MEMOP_LR_W = 5'h08;
localparam MEMOP_SC_W = 5'h09;
localparam MEMOP_AMO  = 5'h0a;
localparam MEMOP_NONE = 5'h10;

localparam BCOND_NEVER  = 2'h0;
localparam BCOND_ALWAYS = 2'h1;
localparam BCOND_ZERO   = 2'h2;
localparam BCOND_NZERO  = 2'h3;

// CSR access types

localparam CSR_WTYPE_W    = 2'h0;
localparam CSR_WTYPE_S    = 2'h1;
localparam CSR_WTYPE_C    = 2'h2;

// Exceptional condition signals which travel alongside (or instead of)
// instructions in the pipeline. These are speculative and can be flushed on
// e.g. branch mispredict. These mostly align with mcause values.

localparam EXCEPT_NONE           = 4'hf;

localparam EXCEPT_INSTR_MISALIGN = 4'h0;
localparam EXCEPT_INSTR_FAULT    = 4'h1;
localparam EXCEPT_INSTR_ILLEGAL  = 4'h2;
localparam EXCEPT_EBREAK         = 4'h3;
localparam EXCEPT_LOAD_ALIGN     = 4'h4;
localparam EXCEPT_LOAD_FAULT     = 4'h5;
localparam EXCEPT_STORE_ALIGN    = 4'h6;
localparam EXCEPT_STORE_FAULT    = 4'h7;
localparam EXCEPT_ECALL_U        = 4'h8;
// MRET, Return from M-mode: not really an exception, but handled like one
localparam EXCEPT_MRET           = 4'ha;
localparam EXCEPT_ECALL_M        = 4'hb;
// spare: c
// spare: d
// REFETCH: flush and refetch sequentially-following instructions, e.g. on
// executing fence.i. Jumps from stage 3 to get ordering against L/S dphase.
localparam EXCEPT_REFETCH        = 4'he;

// Operations for M extension (these are just instr[14:12])

localparam M_OP_MUL    = 3'h0;
localparam M_OP_MULH   = 3'h1;
localparam M_OP_MULHSU = 3'h2;
localparam M_OP_MULHU  = 3'h3;
localparam M_OP_DIV    = 3'h4;
localparam M_OP_DIVU   = 3'h5;
localparam M_OP_REM    = 3'h6;
localparam M_OP_REMU   = 3'h7;
/*****************************************************************************\
|                        Copyright (C) 2022 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// List of addresses for CSRs implemented by Hazard3, including custom CSRs.

// ----------------------------------------------------------------------------
// M-mode CSRs

// Machine Information Registers (RO)
localparam MVENDORID      = 12'hf11; // Vendor ID.
localparam MARCHID        = 12'hf12; // Architecture ID.
localparam MIMPID         = 12'hf13; // Implementation ID.
localparam MHARTID        = 12'hf14; // Hardware thread ID.
localparam MCONFIGPTR     = 12'hf15; // Pointer to configuration data structure.

// Machine Trap Setup (RW)
localparam MSTATUS        = 12'h300; // Machine status register.
localparam MSTATUSH       = 12'h310; // As of priv-1.12 this must be present even if tied 0.
localparam MISA           = 12'h301; // ISA and extensions
localparam MEDELEG        = 12'h302; // Machine exception delegation register.
localparam MIDELEG        = 12'h303; // Machine interrupt delegation register.
localparam MIE            = 12'h304; // Machine interrupt-enable register.
localparam MTVEC          = 12'h305; // Machine trap-handler base address.
localparam MCOUNTEREN     = 12'h306; // Machine counter enable.

// Machine Trap Handling (RW)
localparam MSCRATCH       = 12'h340; // Scratch register for machine trap handlers.
localparam MEPC           = 12'h341; // Machine exception program counter.
localparam MCAUSE         = 12'h342; // Machine trap cause.
localparam MTVAL          = 12'h343; // Machine bad address or instruction.
localparam MIP            = 12'h344; // Machine interrupt pending.

// Machine Memory Protection (RW)
localparam PMPCFG0        = 12'h3a0; // Physical memory protection configuration.
localparam PMPCFG1        = 12'h3a1; // Physical memory protection configuration, RV32 only.
localparam PMPCFG2        = 12'h3a2; // Physical memory protection configuration.
localparam PMPCFG3        = 12'h3a3; // Physical memory protection configuration, RV32 only.
localparam PMPADDR0       = 12'h3b0; // Physical memory protection address register.
localparam PMPADDR1       = 12'h3b1; // ...
localparam PMPADDR2       = 12'h3b2;
localparam PMPADDR3       = 12'h3b3;
localparam PMPADDR4       = 12'h3b4;
localparam PMPADDR5       = 12'h3b5;
localparam PMPADDR6       = 12'h3b6;
localparam PMPADDR7       = 12'h3b7;
localparam PMPADDR8       = 12'h3b8;
localparam PMPADDR9       = 12'h3b9;
localparam PMPADDR10      = 12'h3ba;
localparam PMPADDR11      = 12'h3bb;
localparam PMPADDR12      = 12'h3bc;
localparam PMPADDR13      = 12'h3bd;
localparam PMPADDR14      = 12'h3be;
localparam PMPADDR15      = 12'h3bf;

localparam MSECCFG        = 12'h747;
localparam MSECCFGH       = 12'h757;

// Performance counters (RW)
localparam MCYCLE         = 12'hb00; // Raw cycles since start of day
localparam MINSTRET       = 12'hb02; // Instruction retire count since start of day
localparam MHPMCOUNTER3   = 12'hb03; // WARL (we tie to 0)
localparam MHPMCOUNTER4   = 12'hb04; // WARL (we tie to 0)
localparam MHPMCOUNTER5   = 12'hb05; // WARL (we tie to 0)
localparam MHPMCOUNTER6   = 12'hb06; // WARL (we tie to 0)
localparam MHPMCOUNTER7   = 12'hb07; // WARL (we tie to 0)
localparam MHPMCOUNTER8   = 12'hb08; // WARL (we tie to 0)
localparam MHPMCOUNTER9   = 12'hb09; // WARL (we tie to 0)
localparam MHPMCOUNTER10  = 12'hb0a; // WARL (we tie to 0)
localparam MHPMCOUNTER11  = 12'hb0b; // WARL (we tie to 0)
localparam MHPMCOUNTER12  = 12'hb0c; // WARL (we tie to 0)
localparam MHPMCOUNTER13  = 12'hb0d; // WARL (we tie to 0)
localparam MHPMCOUNTER14  = 12'hb0e; // WARL (we tie to 0)
localparam MHPMCOUNTER15  = 12'hb0f; // WARL (we tie to 0)
localparam MHPMCOUNTER16  = 12'hb10; // WARL (we tie to 0)
localparam MHPMCOUNTER17  = 12'hb11; // WARL (we tie to 0)
localparam MHPMCOUNTER18  = 12'hb12; // WARL (we tie to 0)
localparam MHPMCOUNTER19  = 12'hb13; // WARL (we tie to 0)
localparam MHPMCOUNTER20  = 12'hb14; // WARL (we tie to 0)
localparam MHPMCOUNTER21  = 12'hb15; // WARL (we tie to 0)
localparam MHPMCOUNTER22  = 12'hb16; // WARL (we tie to 0)
localparam MHPMCOUNTER23  = 12'hb17; // WARL (we tie to 0)
localparam MHPMCOUNTER24  = 12'hb18; // WARL (we tie to 0)
localparam MHPMCOUNTER25  = 12'hb19; // WARL (we tie to 0)
localparam MHPMCOUNTER26  = 12'hb1a; // WARL (we tie to 0)
localparam MHPMCOUNTER27  = 12'hb1b; // WARL (we tie to 0)
localparam MHPMCOUNTER28  = 12'hb1c; // WARL (we tie to 0)
localparam MHPMCOUNTER29  = 12'hb1d; // WARL (we tie to 0)
localparam MHPMCOUNTER30  = 12'hb1e; // WARL (we tie to 0)
localparam MHPMCOUNTER31  = 12'hb1f; // WARL (we tie to 0)

localparam MCYCLEH        = 12'hb80; // High halves of each counter
localparam MINSTRETH      = 12'hb82;
localparam MHPMCOUNTER3H  = 12'hb83;
localparam MHPMCOUNTER4H  = 12'hb84;
localparam MHPMCOUNTER5H  = 12'hb85;
localparam MHPMCOUNTER6H  = 12'hb86;
localparam MHPMCOUNTER7H  = 12'hb87;
localparam MHPMCOUNTER8H  = 12'hb88;
localparam MHPMCOUNTER9H  = 12'hb89;
localparam MHPMCOUNTER10H = 12'hb8a;
localparam MHPMCOUNTER11H = 12'hb8b;
localparam MHPMCOUNTER12H = 12'hb8c;
localparam MHPMCOUNTER13H = 12'hb8d;
localparam MHPMCOUNTER14H = 12'hb8e;
localparam MHPMCOUNTER15H = 12'hb8f;
localparam MHPMCOUNTER16H = 12'hb90;
localparam MHPMCOUNTER17H = 12'hb91;
localparam MHPMCOUNTER18H = 12'hb92;
localparam MHPMCOUNTER19H = 12'hb93;
localparam MHPMCOUNTER20H = 12'hb94;
localparam MHPMCOUNTER21H = 12'hb95;
localparam MHPMCOUNTER22H = 12'hb96;
localparam MHPMCOUNTER23H = 12'hb97;
localparam MHPMCOUNTER24H = 12'hb98;
localparam MHPMCOUNTER25H = 12'hb99;
localparam MHPMCOUNTER26H = 12'hb9a;
localparam MHPMCOUNTER27H = 12'hb9b;
localparam MHPMCOUNTER28H = 12'hb9c;
localparam MHPMCOUNTER29H = 12'hb9d;
localparam MHPMCOUNTER30H = 12'hb9e;
localparam MHPMCOUNTER31H = 12'hb9f;

localparam MCOUNTINHIBIT  = 12'h320; // Count inhibit register for mcycle/minstret
localparam MHPMEVENT3     = 12'h323; // WARL (we tie to 0)
localparam MHPMEVENT4     = 12'h324; // WARL (we tie to 0)
localparam MHPMEVENT5     = 12'h325; // WARL (we tie to 0)
localparam MHPMEVENT6     = 12'h326; // WARL (we tie to 0)
localparam MHPMEVENT7     = 12'h327; // WARL (we tie to 0)
localparam MHPMEVENT8     = 12'h328; // WARL (we tie to 0)
localparam MHPMEVENT9     = 12'h329; // WARL (we tie to 0)
localparam MHPMEVENT10    = 12'h32a; // WARL (we tie to 0)
localparam MHPMEVENT11    = 12'h32b; // WARL (we tie to 0)
localparam MHPMEVENT12    = 12'h32c; // WARL (we tie to 0)
localparam MHPMEVENT13    = 12'h32d; // WARL (we tie to 0)
localparam MHPMEVENT14    = 12'h32e; // WARL (we tie to 0)
localparam MHPMEVENT15    = 12'h32f; // WARL (we tie to 0)
localparam MHPMEVENT16    = 12'h330; // WARL (we tie to 0)
localparam MHPMEVENT17    = 12'h331; // WARL (we tie to 0)
localparam MHPMEVENT18    = 12'h332; // WARL (we tie to 0)
localparam MHPMEVENT19    = 12'h333; // WARL (we tie to 0)
localparam MHPMEVENT20    = 12'h334; // WARL (we tie to 0)
localparam MHPMEVENT21    = 12'h335; // WARL (we tie to 0)
localparam MHPMEVENT22    = 12'h336; // WARL (we tie to 0)
localparam MHPMEVENT23    = 12'h337; // WARL (we tie to 0)
localparam MHPMEVENT24    = 12'h338; // WARL (we tie to 0)
localparam MHPMEVENT25    = 12'h339; // WARL (we tie to 0)
localparam MHPMEVENT26    = 12'h33a; // WARL (we tie to 0)
localparam MHPMEVENT27    = 12'h33b; // WARL (we tie to 0)
localparam MHPMEVENT28    = 12'h33c; // WARL (we tie to 0)
localparam MHPMEVENT29    = 12'h33d; // WARL (we tie to 0)
localparam MHPMEVENT30    = 12'h33e; // WARL (we tie to 0)
localparam MHPMEVENT31    = 12'h33f; // WARL (we tie to 0)

// Other standard M-mode CSRs:
localparam MENVCFG        = 12'h30a;
localparam MENVCFGH       = 12'h31a;

// Custom M-mode CSRs:
localparam PMPCFGM0       = 12'hbd0; // Make PMP regions M-mode without locking
//                              bd1  // (reserved for >32 regions)

localparam MEIEA          = 12'hbe0; // External interrupt pending array
localparam MEIPA          = 12'hbe1; // External interrupt enable array
localparam MEIFA          = 12'hbe2; // External interrupt force array
localparam MEIPRA         = 12'hbe3; // External interrupt priority array
localparam MEINEXT        = 12'hbe4; // Next external interrupt
localparam MEICONTEXT     = 12'hbe5; // External interrupt context register

localparam MSLEEP         = 12'hbf0; // M-mode sleep control register
localparam H3MISA         = 12'hbf1; // Hazard3 M-mode ISA identification register

// ----------------------------------------------------------------------------
// U-mode CSRs

// Read-only aliases of M-mode counter CSRs:
localparam CYCLE          = 12'hc00;
localparam TIME           = 12'hc01;
localparam INSTRET        = 12'hc02;
localparam CYCLEH         = 12'hc80;
localparam TIMEH          = 12'hc81;
localparam INSTRETH       = 12'hc82;

// Custom U-mode CSRs
localparam SLEEP          = 12'h8f0; // U-mode subset of M-mode sleep control

// ----------------------------------------------------------------------------
// Trigger Module

localparam TSELECT       = 12'h7a0;
localparam TDATA1        = 12'h7a1;
localparam TDATA2        = 12'h7a2;
localparam TDATA3        = 12'h7a3;
localparam TINFO         = 12'h7a4;
localparam TCONTROL      = 12'h7a5;
localparam MCONTEXT      = 12'h7a8;

// ----------------------------------------------------------------------------
// D-mode CSRs

localparam DCSR           = 12'h7b0;
localparam DPC            = 12'h7b1;
localparam DMDATA0        = 12'hbff; // Custom read/write

localparam X0 = {XLEN{1'b0}};

// ----------------------------------------------------------------------------
// CSR state + update logic
// ----------------------------------------------------------------------------
// Names are (reg)_(field)

// Generic update logic for write/set/clear of an entire CSR:
wire [XLEN-1:0] wdata_update =
		wtype == CSR_WTYPE_C ? rdata & ~wdata :
		wtype == CSR_WTYPE_S ? rdata |  wdata : wdata;

function [XLEN-1:0] update_nonconst;
	input [XLEN-1:0] prev;
	input [XLEN-1:0] nonconst;
begin
		update_nonconst = (wdata_update & nonconst) | (prev & ~nonconst) ;
end
endfunction

wire enter_debug_mode;
wire exit_debug_mode;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		debug_mode <= 1'b0;
	end else if (DEBUG_SUPPORT) begin
		debug_mode <= (debug_mode && !exit_debug_mode) || enter_debug_mode;
	end
end

// Core execution state, 1 -> M-mode, 0 -> U-mode (if implemented)
reg  m_mode;
wire wen_m_mode = wen && (m_mode || debug_mode);
wire ren_m_mode = ren && (m_mode || debug_mode);

// ----------------------------------------------------------------------------
// Standard trap-handling CSRs

wire debug_suppresses_trap_update = DEBUG_SUPPORT && (debug_mode || enter_debug_mode);

wire trapreg_update = trap_enter_vld && trap_enter_rdy && !debug_suppresses_trap_update;
wire trapreg_update_enter = trapreg_update && except != EXCEPT_MRET && except != EXCEPT_REFETCH;
wire trapreg_update_exit = trapreg_update && except == EXCEPT_MRET;

// Local monitor flag cleared on trap exit (but not on debug-mode exit, to
// permit stepping through LR/SC sequences):
assign clear_excl_on_trap_exit = trapreg_update_exit;

reg mstatus_mpie;
reg mstatus_mie;
reg mstatus_mpp; // only MSB is implemented
reg mstatus_mprv;
reg mstatus_tw;

// Spec text from priv-1.12:
// "An MRET or SRET instruction is used to return from a trap in M-mode or
//  S-mode respectively. When executing an xRET instruction, supposing xPP
//  holds the value y, xIE is set to xPIE; the privilege mode is changed to
//  y; xPIE is set to 1; and xPP is set to the least-privileged supported
//  mode (U if U-mode is implemented, else M). If xPP=M, xRET also sets
//  MPRV=0."

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		m_mode <= 1'b1;
		mstatus_mpie <= 1'b0;
		mstatus_mie <= 1'b0;
		mstatus_mpp <= 1'b1;
		mstatus_mprv <= 1'b0;
		mstatus_tw <= 1'b0;
	end else if (!CSR_M_TRAP) begin
		// Explicit tie-off when unimplemented; avoid synthesis warning about
		// flops insensitive to clk. Should match the rst_n case above so that
		// flops are constant-propagated away.
		m_mode <= 1'b1;
		mstatus_mpie <= 1'b0;
		mstatus_mie <= 1'b0;
		mstatus_mpp <= 1'b1;
		mstatus_mprv <= 1'b0;
		mstatus_tw <= 1'b0;
	end else begin
		if (trapreg_update_exit) begin
			mstatus_mpie <= 1'b1;
			mstatus_mie <= mstatus_mpie;
			if (U_MODE) begin
				m_mode <= mstatus_mpp;
				mstatus_mpp <= 1'b0;
				if (!mstatus_mpp) begin
					mstatus_mprv <= 1'b0;
				end
			end
		end else if (trapreg_update_enter) begin
			mstatus_mpie <= mstatus_mie;
			mstatus_mie <= 1'b0;
			if (U_MODE) begin
				m_mode <= 1'b1;
				mstatus_mpp <= m_mode;
			end
		end else if (wen_m_mode && addr == MSTATUS) begin
			mstatus_mpie <= wdata_update[7];
			mstatus_mie  <= wdata_update[3];
			mstatus_mprv <= wdata_update[17];
			// Note only the MSB of MPP is implemented. It reads back as 11 or 00.
			mstatus_mpp  <= wdata_update[12] || !U_MODE;
			mstatus_tw   <= wdata_update[21] && U_MODE;
		end else if (wen_raw && debug_mode && addr == DCSR && U_MODE && DEBUG_SUPPORT) begin
			// Debugger can change/observe core state directly through
			// dcsr.prv (this doesn't affect debugger operation, as all
			// operations have an effective level of M-mode whilst the core
			// is halted for debug.)
			m_mode <= wdata_update[1];
		end
		if (!U_MODE) begin
			// Explicitly tie-off to constant; some synthesis tools don't like
			// it when variables are only sensitive to the reset.
			m_mode <= 1'b1;
			mstatus_mpp <= 1'b1;
		end
	end
end

// Trap all U-mode WFIs if timeout bit is set. Note that the debug spec
// says "The `wfi` instruction acts as a `nop`" during program buffer
// execution, and nops do not trap, so in debug mode we allow WFIs but
// immediately wake them.
assign trap_wfi = mstatus_tw && !(debug_mode || m_mode);

reg [XLEN-1:0] mscratch;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mscratch <= X0;
	end else if (!CSR_M_TRAP) begin
		// Explicit tie-off when unimplemented
		mscratch <= X0;
	end else begin
		if (wen_m_mode && addr == MSCRATCH) begin
			mscratch <= wdata_update;
		end
	end
end

// Trap vector base
reg  [XLEN-1:0] mtvec_reg;
wire [XLEN-1:0] mtvec = mtvec_reg & ({XLEN{1'b1}} << 2);
wire            irq_vector_enable = mtvec_reg[0];

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mtvec_reg <= MTVEC_INIT;
	end else if (!CSR_M_TRAP) begin
		// Explicit tie-off when unimplemented
		mtvec_reg <= MTVEC_INIT;
	end else begin
		if (wen_m_mode && addr == MTVEC) begin
			mtvec_reg <= update_nonconst(mtvec_reg, MTVEC_WMASK);
		end
	end
end

// Exception program counter
reg [XLEN-1:0] mepc;
// mepc only holds values aligned to instruction alignment
localparam MEPC_MASK = {{XLEN-2{1'b1}}, |EXTENSION_C, 1'b0};

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mepc <= X0;
	end else if (!CSR_M_TRAP) begin
		// Explicit tie-off when unimplemented
		mepc <= X0;
	end else begin
		if (trapreg_update_enter) begin
			mepc <= mepc_in & MEPC_MASK;
		end else if (wen_m_mode && addr == MEPC) begin
			mepc <= wdata_update & MEPC_MASK;
		end
	end
end

// Interrupt enable (reserved bits are tied to 0)
reg [XLEN-1:0] mie;
localparam MIE_WMASK = 32'h00000888; // meie, mtie, msie

wire meicontext_clearts;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mie <= X0;
	end else if (!CSR_M_TRAP) begin
		// Explicit tie-off when unimplemented
		mie <= X0;
	end else begin
		if (wen_m_mode && addr == MIE) begin
			mie <= update_nonconst(mie, MIE_WMASK);
		end else if (wen_m_mode && addr == MEICONTEXT) begin
			// meicontext.clearts/mtiesave/msiesave can be used to clear and
			// save standard timer and soft IRQ enables, simultaneously with
			// saving external interrupt context.
			mie[7] <= (mie[7] || wdata_update[3]) && !meicontext_clearts;
			mie[3] <= (mie[3] || wdata_update[2]) && !meicontext_clearts;
		end
	end
end

// Interrupt pending register (assigned later). In our implementation this
// register is entirely read-only.
wire [XLEN-1:0] mip;

// Trap cause registers. The non-constant bits can be written by software, and
// update automatically on trap entry. The `code` field need only be large
// enough to support causes that will be set by hardware, in our case 4 bits.
// (Note this is different to scause which always has a hard minimum of 5
// bits.)
reg        mcause_irq;
reg  [3:0] mcause_code;
wire       mcause_irq_next;
wire [3:0] mcause_code_next;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mcause_irq <= 1'b0;
		mcause_code <= 4'h0;
	end else if (!CSR_M_TRAP) begin
		// Explicit tie-off when unimplemented
		mcause_irq <= 1'b0;
		mcause_code <= 4'h0;
	end else begin
		if (trapreg_update_enter) begin
			mcause_irq <= mcause_irq_next;
			mcause_code <= mcause_code_next;
		end else if (wen_m_mode && addr == MCAUSE) begin
			{mcause_irq, mcause_code} <= {wdata_update[31], wdata_update[3:0]};
		end
	end
end

// ----------------------------------------------------------------------------
// Standard identification CSRs

// mimpid records the Hazard3 release version. This is updated manually with
// each release.

// Set to 1 if this RTL has not been released to the stable branch:
localparam [0:0] MIMPID_PRERELEASE = 1'b0;

// Major version, e.g the 1 in v1.2.3:
localparam [3:0] MIMPID_MAJOR      = 4'd1;
// Minor version, e.g. the 2 in v1.2.3:
localparam [7:0] MIMPID_MINOR      = 8'd1;
// Patch version, e.g. the 3 in v1.2.3:
localparam [7:0] MIMPID_PATCH      = 8'd1;

// Note the eco_version field is connected externally -- on ASIC this may be
// connected to tie cells so it can be incremented following a metal ECO.
wire [31:0] mimpid_rdata = {
	MIMPID_PRERELEASE,
	3'd0,
	MIMPID_MAJOR,
	MIMPID_MINOR,
	MIMPID_PATCH,
	eco_version,
	4'h0
};

localparam [31:0] MISA_VAL = {
	2'h1,              // MXL: 32-bit
	{XLEN-28{1'b0}},   // WLRL

	2'd0,              // Z, Y, no
	1'b1,              // X is always set due to (at least) Xh3misa
	2'd0,              // V, W, no
	|U_MODE,
	7'd0,              // T...N, no
	|EXTENSION_M,
	3'd0,              // L...J, no
	~|EXTENSION_E,     // RVI base ISA
	3'd0,              // H, G, F, no
	|EXTENSION_E,      // RVE base ISA
	1'b0,              // D, no
	|EXTENSION_C,
	&{                 // B is defined as ZbaZbbZbs
		|EXTENSION_ZBA,
		|EXTENSION_ZBB,
		|EXTENSION_ZBS
	},
	|EXTENSION_A
};

// ----------------------------------------------------------------------------
// Custom external IRQ controller

wire [XLEN-1:0] irq_ctrl_rdata;
wire            external_irq_pending;

generate
if (|EXTENSION_XH3IRQ) begin: have_irq_ctrl

	hazard3_irq_ctrl #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
	) irq_ctrl (
		.clk                  (clk),
		.clk_always_on        (clk_always_on),
		.rst_n                (rst_n),

		.addr                 (addr),
		.wtype                (wtype),
		.wen_m_mode           (wen_m_mode),
		.ren_m_mode           (ren_m_mode),
		.wdata_raw            (wdata),
		.wdata                (wdata_update),
		.rdata                (irq_ctrl_rdata),

		.trapreg_update_enter (trapreg_update_enter),
		.trapreg_update_exit  (trapreg_update_exit),
		.trap_entry_is_eirq   (mcause_irq_next && mcause_code_next == 4'hb),

		.meicontext_clearts   (meicontext_clearts),
		.mie_mtie             (mie[7]),
		.mie_msie             (mie[3]),

		.irq                  (irq),
		.external_irq_pending (external_irq_pending)
	);

end else begin: no_irq_ctrl

	reg external_irq_pending_r;

	always @ (posedge clk_always_on or negedge rst_n) begin
		if (!rst_n) begin
			external_irq_pending_r <= 1'b0;
		end else begin
			external_irq_pending_r <= |irq;
		end
	end

	assign irq_ctrl_rdata = {W_DATA{1'b0}};
	assign meicontext_clearts = 1'b0;
	assign external_irq_pending = external_irq_pending_r;

end
endgenerate

// ----------------------------------------------------------------------------
// Custom sleep/power control CSRs

reg msleep_sleeponblock;
reg msleep_powerdown;
reg msleep_deepsleep;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		msleep_sleeponblock <= 1'b0;
		msleep_powerdown    <= 1'b0;
		msleep_deepsleep    <= 1'b0;
	end else if (wen_m_mode && addr == MSLEEP) begin
		msleep_sleeponblock <= wdata_update[2] && |EXTENSION_XH3POWER;
		msleep_powerdown    <= wdata_update[1] && |EXTENSION_XH3POWER;
		msleep_deepsleep    <= wdata_update[0] && |EXTENSION_XH3POWER;
	end
end

assign pwr_allow_sleep_on_block = msleep_sleeponblock;
assign pwr_allow_power_down = msleep_powerdown;
assign pwr_allow_clkgate = msleep_deepsleep;

// ----------------------------------------------------------------------------
// Custom identification CSRs

// Derive implied extensions:

// B is a shorthand for ZbaZbbZbs:
localparam [0:0]   EXTENSION_B     = |EXTENSION_ZBA && |EXTENSION_ZBB && |EXTENSION_ZBS;
// RV32I and RV32E are complementary:
localparam [0:0]   EXTENSION_I     = ~|EXTENSION_E;
// Zbkc is a subset of Zbc:
localparam [0:0]   EXTENSION_ZBKC  = |EXTENSION_ZBC;
// Zca is (instructions-wise) a subset of C:
localparam [0:0]   EXTENSION_ZCA   = |EXTENSION_C;
// We have data-independent timing for all Zkt instructions except for mulh,
// mulhsu executed on the sequential multiply/divide circuit:
localparam [0:0]   EXTENSION_ZKT   = !(|EXTENSION_M && !(|MUL_FAST && |MULH_FAST));
// Zmmul is (instructions-wise) a subset of M:
localparam [0:0]   EXTENSION_ZMMUL = |EXTENSION_M;

localparam [31:0]  h3misa_standard_len = 32'd77;

localparam [127:0] h3misa_standard_extensions =
	({127'd0, |EXTENSION_A       } << 0 ) |
	({127'd0, |EXTENSION_B       } << 1 ) |
	({127'd0, |EXTENSION_C       } << 2 ) |
	({127'd0, |EXTENSION_E       } << 4 ) |
	({127'd0, |EXTENSION_I       } << 8 ) |
	({127'd0, |EXTENSION_M       } << 12) |
	({127'd0, |EXTENSION_ZBA     } << 27) |
	({127'd0, |EXTENSION_ZBB     } << 28) |
	({127'd0, |EXTENSION_ZBC     } << 29) |
	({127'd0, |EXTENSION_ZBKB    } << 30) |
	({127'd0, |EXTENSION_ZBC     } << 31) |
	({127'd0, |EXTENSION_ZBKX    } << 32) |
	({127'd0, |EXTENSION_ZBS     } << 33) |
	({127'd0, |EXTENSION_ZKT     } << 46) |
	({127'd0, |EXTENSION_C       } << 66) |
	({127'd0, |EXTENSION_ZCB     } << 67) |
	({127'd0, |EXTENSION_ZILSD   } << 72) |
	({127'd0, |EXTENSION_ZCLSD   } << 73) |
	({127'd0, |EXTENSION_ZCMP    } << 74) |
	({127'd0, |EXTENSION_ZIFENCEI} << 75) |
	({127'd0, |EXTENSION_ZMMUL   } << 76) |
	128'd0;

localparam [31:0]  h3misa_custom_len = 32'd5;

localparam [255:0] h3misa_custom_extensions = {
	32'd0,                                       // Reserved
	32'd0,                                       // Reserved
	32'd0,                                       // Reserved
	32'h01_00_00_00 & {32{|EXTENSION_XH3BEXTM}}, // Xh3bextm
	32'h01_00_00_00 & {32{|EXTENSION_XH3POWER}}, // Xh3power
	32'h01_00_00_00 & {32{|EXTENSION_XH3PMPM}},  // Xh3pmpm
	32'h01_00_00_00 & {32{|EXTENSION_XH3IRQ}},   // Xh3irq
	32'h01_00_00_00 & {32{|CSR_M_MANDATORY}}     // Xh3misa
};

wire [63:0] h3misa_info = {
	h3misa_custom_len,
	h3misa_standard_len
};

wire [31:0]  h3misa_rdata =
	!wdata[10] ? h3misa_standard_extensions[32 * wdata[1:0] +: 32] :
	!wdata[8]  ? h3misa_info               [32 * wdata[  0] +: 32] :
	             h3misa_custom_extensions  [32 * wdata[2:0] +: 32];

// ----------------------------------------------------------------------------
// Counters

reg mcountinhibit_cy;
reg mcountinhibit_ir;

reg [XLEN-1:0] mcycleh;
reg [XLEN-1:0] mcycle;
reg [XLEN-1:0] minstreth;
reg [XLEN-1:0] minstret;

// Writing to either half suppresses increment of the entire 64-bit register.
// It doesn't just replace the value of one 32-bit half. See:
//   https://github.com/riscv/riscv-isa-manual/issues/1255
wire mcycle_stopped = mcountinhibit_cy || debug_mode || wen_m_mode && (
	addr == MCYCLEH || addr == MCYCLE
);

wire minstret_stopped = mcountinhibit_ir || debug_mode || wen_m_mode && (
	addr == MINSTRETH || addr == MINSTRET
);

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mcycleh <= X0;
		mcycle <= X0;
		minstreth <= X0;
		minstret <= X0;
		// Counters inhibited by default to save energy
		mcountinhibit_cy <= 1'b1;
		mcountinhibit_ir <= 1'b1;
	end else begin
		if (!mcycle_stopped) begin
			{mcycleh, mcycle} <=
				({mcycleh, mcycle} + 64'd1) & {2*XLEN{|CSR_COUNTER}};
		end
		if (instr_ret && !minstret_stopped) begin
			{minstreth, minstret} <=
				({minstreth, minstret} + 64'd1) & {2*XLEN{|CSR_COUNTER}};
		end
		if (wen_m_mode) begin
			if (addr == MCYCLEH) begin
				mcycleh <= wdata_update & {XLEN{|CSR_COUNTER}};
			end else if (addr == MCYCLE) begin
				mcycle <= wdata_update & {XLEN{|CSR_COUNTER}};
			end else if (addr == MINSTRETH) begin
				minstreth <= wdata_update & {XLEN{|CSR_COUNTER}};
			end else if (addr == MINSTRET) begin
				minstret <= wdata_update & {XLEN{|CSR_COUNTER}};
			end else if (addr == MCOUNTINHIBIT) begin
				{mcountinhibit_ir, mcountinhibit_cy} <= {wdata_update[2], wdata_update[0]} | {2{~|CSR_COUNTER}};
			end
		end
	end
end

// Note we still implement tm, even though the time/timeh CSRs are
// unimplemented. The tm bit provides a standard way to configure whether
// M-mode software permits the trapped access to reach the timer registers.
// (This is in fact required by the RVM-CSI platform spec.)
reg mcounteren_cy;
reg mcounteren_tm;
reg mcounteren_ir;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mcounteren_cy <= 1'b0;
		mcounteren_tm <= 1'b0;
		mcounteren_ir <= 1'b0;
	end else if (wen_m_mode && addr == MCOUNTEREN) begin
		mcounteren_cy <= wdata_update[0] && |CSR_COUNTER && |U_MODE;
		mcounteren_tm <= wdata_update[1] && |CSR_COUNTER && |U_MODE;
		mcounteren_ir <= wdata_update[2] && |CSR_COUNTER && |U_MODE;
	end
end

// ----------------------------------------------------------------------------
// Debug-mode CSRs

// The following DCSR bits are read/writable as normal:
// - ebreakm (bit 15)
// - ebreaku (bit 12) if U-mode is supported
// - step (bit 2)
// The following are read-only volatile:
// - cause (bits 8:6)
// All others are hardwired constants.

reg dcsr_ebreakm;
reg dcsr_ebreaku;
reg dcsr_step;
reg [2:0] dcsr_cause;
wire [2:0] dcause_next;

localparam DCSR_CAUSE_EBREAK  = 3'h1;
localparam DCSR_CAUSE_TRIGGER = 3'h2;
localparam DCSR_CAUSE_HALTREQ = 3'h3;
localparam DCSR_CAUSE_STEP    = 3'h4;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		dcsr_ebreakm <= 1'b0;
		dcsr_ebreaku <= 1'b0;
		dcsr_step <= 1'b0;
		dcsr_cause <= 3'h0;
	end else begin
		// Note we can use the ungated write enable in debug mode since a CSR
		// write to a valid D-mode CSR address is guaranteed not to generate
		// an exception (of any kind, including PMP) in D-mode.
		if (debug_mode && wen_raw && addr == DCSR) begin
			{dcsr_ebreakm, dcsr_ebreaku, dcsr_step} <= {
				wdata_update[15],
				wdata_update[12] && U_MODE,
				wdata_update[2]
			} & {3{|DEBUG_SUPPORT}};
		end
		if (enter_debug_mode) begin
			dcsr_cause <= dcause_next & {3{|DEBUG_SUPPORT}};
		end
	end
end

// DPC is mapped to the real PC in the decode module. We set it to the
// exception return address when entering Debug Mode, and whilst in Debug
// Mode we can write to it as though it were a CSR. Note only IALIGN'd values
// can be written to dpc.
assign debug_dpc_wdata = (debug_mode ? wdata_update           : mepc_in) & (~X0 << 2 - EXTENSION_C);
assign debug_dpc_wen   =  debug_mode ? wen_raw && addr == DPC : enter_debug_mode;

// Debug Module's data0 register is mapped into the core's CSR space:
assign dbg_data0_wdata = wdata;
assign dbg_data0_wen = debug_mode && wen_raw && addr == DMDATA0;

reg tcontrol_mte;
reg tcontrol_mpte;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		tcontrol_mpte <= 1'b0;
		tcontrol_mte <= 1'b0;
	end else if (wen_m_mode && addr == TCONTROL) begin
		tcontrol_mpte <= wdata_update[7] && DEBUG_SUPPORT;
		tcontrol_mte <= wdata_update[3] && DEBUG_SUPPORT;
	end else if (DEBUG_SUPPORT && trapreg_update_enter) begin
		tcontrol_mte <= 1'b0;
		tcontrol_mpte <= tcontrol_mte;
	end else if (DEBUG_SUPPORT && trapreg_update_exit) begin
		tcontrol_mte <= tcontrol_mpte;
		// Unlike mstatus.mie/mpie, tcontrol.mpte is unchanged by trap exit.
	end
end

// ----------------------------------------------------------------------------
// Read port + detect addressing of unmapped CSRs
// ----------------------------------------------------------------------------

reg decode_match;

// Address match conditions -- certain CSRs are inaccessible if their
// conditions are not met:
wire match_drw = DEBUG_SUPPORT && debug_mode;
wire match_mrw = m_mode || debug_mode;
wire match_mro = (m_mode || debug_mode) && !wen_soon;
wire match_urw = U_MODE && 1'b1;
wire match_uro = U_MODE && !wen_soon;

always @ (*) begin
	decode_match = 1'b0;
	rdata = {XLEN{1'b0}};
	pmp_cfg_wen = 1'b0;
	trig_cfg_wen = 1'b0;
	case (addr)

	// ------------------------------------------------------------------------
	// Mandatory CSRs

	MISA: if (CSR_M_MANDATORY) begin
		// WARL, so it is legal to be tied constant
		decode_match = match_mrw;
		rdata = MISA_VAL;
	end
	MVENDORID: if (CSR_M_MANDATORY) begin
		decode_match = match_mro;
		rdata = MVENDORID_VAL;
	end
	MARCHID: if (CSR_M_MANDATORY) begin
		decode_match = match_mro;
		// Hazard3's open source architecture ID
		rdata = 32'd27;
	end
	MIMPID: if (CSR_M_MANDATORY) begin
		decode_match = match_mro;
		rdata = mimpid_rdata;
	end
	MHARTID: if (CSR_M_MANDATORY) begin
		decode_match = match_mro;
		rdata = mhartid_val;
	end

	MCONFIGPTR: if (CSR_M_MANDATORY) begin
		decode_match = match_mro;
		rdata = MCONFIGPTR_VAL;
	end

	MSTATUS: if (CSR_M_MANDATORY || CSR_M_TRAP) begin
		decode_match = match_mrw;
		rdata = {
			1'b0,             // Never any dirty state besides GPRs
			8'd0,             // (WPRI)
			1'b0,             // TSR (Trap SRET), tied 0 if no S mode.
			mstatus_tw,       // TW (Timeout Wait)
			1'b0,             // TVM (trap virtual memory), tied 0 if no S mode.
			1'b0,             // MXR (Make eXecutable Readable), tied 0 if no S mode.
			1'b0,             // SUM, tied 0 if no S mode
			mstatus_mprv,     // MPRV (modify privilege)
			4'd0,             // XS, FS always "off" (no extension state to clear!)
			{2{mstatus_mpp}}, // MPP (M-mode previous privilege), only M and U supported
			2'd0,             // (WPRI)
			1'b0,             // SPP, tied 0 if S mode not supported
			mstatus_mpie,     // Previous interrupt enable
			3'd0,             // No S, U
			mstatus_mie,      // Interrupt enable
			3'd0              // No S, U
		};
	end

	// MSTATUSH is all zeroes (fields are MBE and SBE, which are zero because
	// we are pure little-endian.) Prior to priv-1.12 MSTATUSH could be left
	// unimplemented in this case, but now it must be decoded even if
	// hardwired to 0.
	MSTATUSH: if (CSR_M_MANDATORY || CSR_M_TRAP) begin
		decode_match = match_mrw;
	end

	// MEDELEG, MIDELEG should not exist for no-S-mode implementations. Will
	// raise illegal instruction exception if accessed.

	// MENVCFG is seemingly mandatory as of M-mode v1.12, if U-mode is
	// implemented. All of its fields are tied to 0 in our implementation.
	MENVCFGH: if (U_MODE) begin
		decode_match = match_mrw;
	end
	MENVCFG: if (U_MODE) begin
		decode_match = match_mrw;
	end

	// ------------------------------------------------------------------------
	// Trap-handling CSRs

	// This is a 32 bit synthesised register with set/clear/write/read, don't
	// turn it on unless we really have to
	MSCRATCH: if (CSR_M_TRAP && CSR_M_MANDATORY) begin
		decode_match = match_mrw;
		rdata = mscratch;
	end

	MEPC: if (CSR_M_TRAP) begin
		decode_match = match_mrw;
		rdata = mepc;
	end

	MCAUSE: if (CSR_M_TRAP) begin
		decode_match = match_mrw;
		rdata = {
			mcause_irq,      // Sign bit is 1 for IRQ, 0 for exception
			{27{1'b0}},      // Padding
			mcause_code[3:0] // Enough for 16 external IRQs, which is all we have room for in mip/mie
		};
	end

	MTVAL: if (CSR_M_TRAP) begin
		decode_match = match_mrw;
		// Hardwired to 0
	end

	MIE: if (CSR_M_TRAP) begin
		decode_match = match_mrw;
		rdata = mie;
	end

	MIP: if (CSR_M_TRAP) begin
		// Writes are permitted, but ignored.
		decode_match = match_mrw;
		rdata = mip;
	end

	MTVEC: if (CSR_M_TRAP) begin
		decode_match = match_mrw;
		rdata = {
			mtvec[XLEN-1:2],  // BASE
			1'b0,             // Reserved mode bit gets WARL'd to 0
			irq_vector_enable // MODE is vectored (2'h1) or direct (2'h0)
		};
	end

	// ------------------------------------------------------------------------
	// Counter CSRs

	// Get the tied WARLs out the way first
	MHPMCOUNTER3:   if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER4:   if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER5:   if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER6:   if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER7:   if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER8:   if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER9:   if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER10:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER11:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER12:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER13:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER14:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER15:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER16:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER17:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER18:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER19:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER20:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER21:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER22:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER23:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER24:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER25:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER26:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER27:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER28:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER29:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER30:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER31:  if (CSR_COUNTER) begin decode_match = match_mrw; end

	MHPMCOUNTER3H:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER4H:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER5H:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER6H:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER7H:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER8H:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER9H:  if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER10H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER11H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER12H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER13H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER14H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER15H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER16H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER17H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER18H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER19H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER20H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER21H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER22H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER23H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER24H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER25H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER26H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER27H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER28H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER29H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER30H: if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMCOUNTER31H: if (CSR_COUNTER) begin decode_match = match_mrw; end

	MHPMEVENT3:     if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT4:     if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT5:     if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT6:     if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT7:     if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT8:     if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT9:     if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT10:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT11:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT12:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT13:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT14:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT15:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT16:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT17:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT18:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT19:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT20:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT21:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT22:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT23:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT24:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT25:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT26:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT27:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT28:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT29:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT30:    if (CSR_COUNTER) begin decode_match = match_mrw; end
	MHPMEVENT31:    if (CSR_COUNTER) begin decode_match = match_mrw; end

	MCOUNTINHIBIT:  if (CSR_COUNTER) begin
		decode_match = match_mrw;
		rdata = {
			29'd0,
			mcountinhibit_ir,
			1'b0,
			mcountinhibit_cy
		};
	end

	MCYCLE: if (CSR_COUNTER) begin
		decode_match = match_mrw;
		rdata = mcycle;
	end
	MINSTRET: if (CSR_COUNTER) begin
		decode_match = match_mrw;
		rdata = minstret;
	end

	MCYCLEH: if (CSR_COUNTER) begin
		decode_match = match_mrw;
		rdata = mcycleh;
	end
	MINSTRETH: if (CSR_COUNTER) begin
		decode_match = match_mrw;
		rdata = minstreth;
	end

	MCOUNTEREN: if (CSR_COUNTER && U_MODE) begin
		decode_match = match_mrw;
		rdata = {
			29'd0,
			mcounteren_ir,
			mcounteren_tm,
			mcounteren_cy
		};
	end

	// ------------------------------------------------------------------------
	// PMP CSRs (bridge to PMP config interface)

	// If PMP is present, all 16 registers are present, but some may be WARL'd
	// to 0 depending on how many regions are actually implemented.
	PMPCFG0:   if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPCFG1:   if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPCFG2:   if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPCFG3:   if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR0:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR1:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR2:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR3:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR4:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR5:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR6:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR7:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR8:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR9:  if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR10: if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR11: if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR12: if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR13: if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR14: if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end
	PMPADDR15: if (PMP_REGIONS > 0) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end

	// MSECCFG is strictly optional, and we don't implement any of its
	// features (ePMP etc) so we don't decode it.

	// ------------------------------------------------------------------------
	// U-mode CSRs

	// The read-only counters are always visible to M mode, and are visible to
	// U mode if the corresponding mcounteren bit is set.
	CYCLE: if (CSR_COUNTER) begin
		decode_match = mcounteren_cy ? match_uro : match_mro;
		rdata = mcycle;
	end
	CYCLEH: if (CSR_COUNTER) begin
		decode_match = mcounteren_cy ? match_uro : match_mro;
		rdata = mcycleh;
	end

	INSTRET: if (CSR_COUNTER) begin
		decode_match = mcounteren_ir ? match_uro : match_mro;
		rdata = minstret;
	end
	INSTRETH: if (CSR_COUNTER) begin
		decode_match = mcounteren_ir ? match_uro : match_mro;
		rdata = minstreth;
	end

	// ------------------------------------------------------------------------
	// Trigger Module CSRs

	// When debug support is enabled, we always have basic trigger support
	// (instruction count, interrupt and exception). Breakpoints are optional.
	TSELECT: if (DEBUG_SUPPORT) begin
		decode_match = match_mrw;
		trig_cfg_wen = match_mrw && wen;
		rdata = trig_cfg_rdata;
	end

	TDATA1: if (DEBUG_SUPPORT) begin
		decode_match = match_mrw;
		trig_cfg_wen = match_mrw && wen;
		rdata = trig_cfg_rdata;
	end

	TDATA2: if (DEBUG_SUPPORT) begin
		decode_match = match_mrw;
		trig_cfg_wen = match_mrw && wen;
		rdata = trig_cfg_rdata;
	end

	TDATA3: if (DEBUG_SUPPORT) begin
		// Unimplemented but must be accessible, so hardwired to 0.
		decode_match = match_mrw;
		rdata = 32'h00000000;
	end

	TINFO: if (DEBUG_SUPPORT) begin
		// Note tinfo is a read-write CSR (writes don't trap) even though it
		// is entirely read-only.
		decode_match = match_mrw;
		trig_cfg_wen = match_mrw && wen;
		rdata = trig_cfg_rdata;
	end

	TCONTROL: if (DEBUG_SUPPORT) begin
		decode_match = match_mrw;
		rdata = {
			24'h0,
			tcontrol_mpte,
			3'h0,
			tcontrol_mte,
			3'h0
		};
	end

	// ------------------------------------------------------------------------
	// Debug CSRs

	DCSR: if (DEBUG_SUPPORT) begin
		decode_match = match_drw;
		rdata = {
			4'h4,         // xdebugver = 4, for 0.13.2 debug spec
			12'd0,        // reserved
			dcsr_ebreakm,
			2'h0,         // No mode 2/1
			dcsr_ebreaku,
			1'b0,         // stepie = 0, no interrupts in single-step mode
			1'b1,         // stopcount = 1, no counter increment in debug mode
			1'b1,         // stoptime = 0, no core-local timer increment in debug mode
			dcsr_cause,
			1'b0,         // reserved
			1'b0,         // mprven = 0, debugger always acts as M-mode
			1'b0,         // nmip = 0, we have no NMI
			dcsr_step,
			{2{m_mode}}   // priv = M or U, report the core state directly
		};
	end

	DPC: if (DEBUG_SUPPORT) begin
		decode_match = match_drw;
		rdata = debug_dpc_rdata;
	end

	DMDATA0: if (DEBUG_SUPPORT) begin
		decode_match = match_drw;
		rdata = dbg_data0_rdata;
	end

	// ------------------------------------------------------------------------
	// Custom CSRs

	PMPCFGM0: if (PMP_REGIONS > 0 && EXTENSION_XH3PMPM) begin
		decode_match = match_mrw;
		pmp_cfg_wen = match_mrw && wen;
		rdata = pmp_cfg_rdata;
	end

	MEIEA: if (CSR_M_TRAP && EXTENSION_XH3IRQ) begin
		decode_match = match_mrw;
		rdata = irq_ctrl_rdata;
	end

	MEIPA: if (CSR_M_TRAP && EXTENSION_XH3IRQ) begin
		decode_match = match_mrw;
		rdata = irq_ctrl_rdata;
	end

	MEIFA: if (CSR_M_TRAP && EXTENSION_XH3IRQ) begin
		decode_match = match_mrw;
		rdata = irq_ctrl_rdata;
	end

	MEIPRA: if (CSR_M_TRAP && EXTENSION_XH3IRQ) begin
		decode_match = match_mrw;
		rdata = irq_ctrl_rdata;
	end

	MEINEXT: if (CSR_M_TRAP && EXTENSION_XH3IRQ) begin
		decode_match = match_mrw;
		rdata = irq_ctrl_rdata;
	end

	MEICONTEXT: if (CSR_M_TRAP && EXTENSION_XH3IRQ) begin
		decode_match = match_mrw;
		rdata = irq_ctrl_rdata;
	end

	MSLEEP: if (EXTENSION_XH3POWER) begin
		decode_match = match_mrw;
		rdata = {
			29'h0,
			msleep_sleeponblock,
			msleep_powerdown,
			msleep_deepsleep
		};
	end

	H3MISA: if (CSR_M_MANDATORY) begin
		decode_match = match_mrw;
		rdata = h3misa_rdata;
	end

	default: begin end
	endcase
end

assign illegal = (wen_soon || ren_soon) && !decode_match;

// Trigger refetch of sequentially-next instructions on certain CSR updates:
assign write_is_fetch_ordered = wen_raw && !debug_mode && (
	(PMP_REGIONS   > 0 && addr >= PMPADDR0 && addr <= PMPADDR15) ||
	(PMP_REGIONS   > 0 && addr >= PMPCFG0  && addr <= PMPCFG3  ) ||
	(DEBUG_SUPPORT > 0 && addr == TDATA1                       ) ||
	(DEBUG_SUPPORT > 0 && addr == TDATA2                       ) ||
	(DEBUG_SUPPORT > 0 && addr == TCONTROL                     )
);

// ----------------------------------------------------------------------------
// Debug run/halt

// req_resume_prev is to cut an in->out path from request to trap addr.
reg have_just_reset;
reg step_halt_req;
reg dbg_req_resume_prev;
reg dbg_req_halt_prev;
reg dbg_req_halt_on_reset_sticky;
reg pending_dbg_resume_prev;

wire pending_dbg_resume = (pending_dbg_resume_prev || dbg_req_resume_prev) && debug_mode;

// Note exception entry is also considered a step -- in this case we are
// supposed to take the trap and then break to debug mode before executing the
// first trap instruction. *IRQ* entry does not occur when step is 1 (because
// we hardwire dcsr.stepie to 0)
wire step_event = instr_ret || (trap_enter_vld && trap_enter_rdy);

// Halt request input register needs to always be clocked, because a WFI needs
// to fall through if the debugger requests halt of a sleeping core.
always @ (posedge clk_always_on or negedge rst_n) begin
	if (!rst_n) begin
		dbg_req_halt_prev <= 1'b0;
	end else begin
		// Just a delayed version of the request from outside of the core.
		// Delay is fine because the DM awaits ack before deasserting.
		dbg_req_halt_prev <= dbg_req_halt && DEBUG_SUPPORT != 0;
	end
end

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		have_just_reset <= |DEBUG_SUPPORT;
		dbg_req_halt_on_reset_sticky <= 1'b0;
		step_halt_req <= 1'b0;
		dbg_req_resume_prev <= 1'b0;
		pending_dbg_resume_prev <= 1'b0;
	end else if (DEBUG_SUPPORT) begin
		have_just_reset <= 1'b0;

		// One-cycle delay on assertion of reset halt req is harmless because
		// of a matching delay on the first instruction fetch (reset_holdoff).
		if (have_just_reset && dbg_req_halt_on_reset) begin
			dbg_req_halt_on_reset_sticky <= 1'b1;
		end else if (enter_debug_mode) begin
			dbg_req_halt_on_reset_sticky <= 1'b0;
		end

		dbg_req_resume_prev <= dbg_req_resume;

		if (debug_mode) begin
			step_halt_req <= 1'b0;
		end else if (dcsr_step && step_event) begin
			step_halt_req <= 1'b1;
		end

		pending_dbg_resume_prev <= pending_dbg_resume;
	end
end

// We can enter the halted state in an IRQ-like manner (squeeze in between the
// instructions in stage 2 and stage 3) or in an exception-like manner
// (replace the instruction in stage 3).
//
// Halt request and step-break are IRQ-like. We need to be careful when a halt
// request lines up with an instruction in M which either has generated an
// exception (e.g. an ecall) or may yet generate an exception (a load). In
// this case the correct thing to do is to:
//
// - Squash whatever instruction may be in X, and inhibit X PC increment
// - Wait until after the exception entry is taken (or the load/store
//   completes successfully)
// - Immediately trigger an IRQ-like debug entry.
//
// This ensures the debugger sees mcause/mepc set correctly, with dpc pointing
// to the handler entry point, if the instruction excepts.

wire exception_req_any;
wire halt_delayed_by_exception = exception_req_any || loadstore_dphase_pending;

wire want_halt_except = DEBUG_SUPPORT && !debug_mode && (
	dcsr_ebreakm &&  m_mode && except == EXCEPT_EBREAK ||
	dcsr_ebreaku && !m_mode && except == EXCEPT_EBREAK ||
	except_to_d_mode        && except == EXCEPT_EBREAK
);

// Note exception-like causes (trigger, ebreak) are higher priority than
// IRQ-like.
//
// We must mask halt_req with delay_irq_entry (true on cycle 2 and beyond of
// load/store address phase) because at that point we can't suppress the bus
// access. Others are fine because they are never mid-instruction in stage 2.
wire want_halt_irq_if_no_exception = DEBUG_SUPPORT && !debug_mode && !want_halt_except && (
	(dbg_req_halt_prev && !delay_irq_entry) ||
	dbg_req_halt_on_reset_sticky ||
	step_halt_req
);

// Exception (or potential exception due to load/store) in M delays halt
// entry. The halt intention still blocks X, so we can't get blocked forever
// by a string of load/stores. This is just here to get sequencing between an
// exception (or potential exception) in M, and debug mode entry.
wire want_halt_irq = want_halt_irq_if_no_exception && !halt_delayed_by_exception;

assign dcause_next =
	except == EXCEPT_EBREAK && except_to_d_mode ? 3'h2 : // trigger (priority 4)
	except == EXCEPT_EBREAK                     ? 3'h1 : // ebreak (priority 3)
	dbg_req_halt_prev || dbg_req_halt_on_reset  ? 3'h3 : // halt or reset-halt (priority 1, 2)
	                                              3'h4;  // single-step (priority 0)

wire trap_is_debug_entry = |DEBUG_SUPPORT && !debug_mode && (want_halt_irq || want_halt_except);
assign enter_debug_mode = trap_is_debug_entry && trap_enter_rdy;

assign exit_debug_mode = debug_mode && pending_dbg_resume && trap_enter_rdy;

// Report back to DM instruction injector to tell it its instruction sequence
// has finished (ebreak) or crashed out
assign dbg_instr_caught_ebreak = debug_mode && except == EXCEPT_EBREAK && trap_enter_rdy;

// Note we exclude ebreak from here regardless of dcsr.ebreakm, since we are
// already in debug mode at this point
assign dbg_instr_caught_exception = debug_mode && except != EXCEPT_NONE &&
	except != EXCEPT_EBREAK && trap_enter_rdy;

// ----------------------------------------------------------------------------
// Trap request generation

reg irq_software_r;
reg irq_timer_r;

// Always clocked, as it's used to generate a wakeup.
always @ (posedge clk_always_on or negedge rst_n) begin
	if (!rst_n) begin
		irq_software_r <= 1'b0;
		irq_timer_r <= 1'b0;
	end else begin
		irq_software_r <= irq_software;
		irq_timer_r <= irq_timer;
	end
end

assign mip = {
	20'h0,                // Reserved
	external_irq_pending, // meip, Global pending bit for external IRQs
	3'h0,                 // Reserved
	irq_timer_r,          // mtip, interrupt from memory-mapped timer peripheral
	3'h0,                 // Reserved
	irq_software_r,       // msip, software interrupt from memory-mapped register
	3'h0                  // Reserved
};

wire irq_active = |(mip & mie) && (mstatus_mie || !m_mode) && !dcsr_step;

// WFI clear respects individual interrupt enables but ignores mstatus.mie.
// Additionally, wfi is treated as a nop during single-stepping and D-mode.
// Note that the IRQs and debug halt request input registers are clocked by
// clk_always_on, so that a wakeup can be generated when asleep. Note also
// that want_halt_irq_if_no_exception can be masked while asleep, so also
// feed in the raw halt request as a wakeup source.
assign pwr_wfi_wakeup_req = |(mip & mie) || dcsr_step || debug_mode ||
	want_halt_irq_if_no_exception || dbg_req_halt_prev;

// Priority order from priv spec: external > software > timer
wire [3:0] standard_irq_num =
	mip[11] && mie[11] ? 4'd11 :
	mip[3]  && mie[3]  ? 4'd3  :
	mip[7]  && mie[7]  ? 4'd7  : 4'd0;

// ebreak may be treated as a halt-to-debugger or a regular M-mode exception,
// depending on dcsr.ebreakm.
assign exception_req_any = except != EXCEPT_NONE && !(
	except == EXCEPT_EBREAK && (m_mode ? dcsr_ebreakm : dcsr_ebreaku)
);

wire [3:0] mcause_irq_num =	irq_active ? standard_irq_num : 4'd0;

wire [3:0] vector_sel =	!exception_req_any && irq_vector_enable ? mcause_irq_num : 4'd0;

// Note in the refetch case we're relying on the aliasing of dpc and pc
// (The intent is to fetch the sequentially-next instruction again)
assign trap_addr =
	except == EXCEPT_MRET    ? mepc            :
	except == EXCEPT_REFETCH ? debug_dpc_rdata :
	pending_dbg_resume       ? debug_dpc_rdata :
	                           mtvec | {26'h0, vector_sel, 2'h0};

// Check for exception-like or IRQ-like trap entry; any debug mode entry takes
// priority over any regular trap.
assign trap_is_irq = DEBUG_SUPPORT && (want_halt_except || want_halt_irq) ?
	!want_halt_except : !exception_req_any;

// When there is a loadstore dphase pending, we block off the IRQ trap
// request, but still assert the trap_enter_soon flag. This blocks issue of
// further load/stores which have not yet been locked into aphase, so the IRQ
// can eventually go through. It's not safe to enter an IRQ when a dphase is
// in progress, because that dphase could subsequently except, and sample the
// IRQ vector PC instead of the load/store instruction PC.

assign trap_enter_vld =
	CSR_M_TRAP && (exception_req_any ||
		!delay_irq_entry && !debug_mode && irq_active && !loadstore_dphase_pending) ||
	DEBUG_SUPPORT && (
		(!delay_irq_entry && want_halt_irq) || want_halt_except || pending_dbg_resume);

assign trap_enter_soon = trap_enter_vld || (
	|DEBUG_SUPPORT && want_halt_irq_if_no_exception ||
	!delay_irq_entry && !debug_mode && irq_active && loadstore_dphase_pending
);

assign mcause_irq_next = !exception_req_any;
assign mcause_code_next = exception_req_any ? except : mcause_irq_num;

// Report trap entry to trigger unit
assign trig_event_exception = |DEBUG_SUPPORT && trapreg_update_enter && !trap_is_irq;
assign trig_event_interrupt = |DEBUG_SUPPORT && trapreg_update_enter &&  trap_is_irq;
assign trig_event_trap_cause = {4{|DEBUG_SUPPORT}} & mcause_code_next;

// ----------------------------------------------------------------------------
// Privilege state outputs

assign pmp_cfg_addr = addr;
assign pmp_cfg_wdata = wdata_update;

assign trig_cfg_addr = addr;
assign trig_cfg_wdata = wdata_update;
assign trig_m_en = tcontrol_mte;

// Effective privilege for execution. Used for:
// - Privilege level of branch target fetches (frontend keeps fetch privilege
//   constant during sequential fetch)
// - Checking PC against PMP execute permission

assign m_mode_execution = !U_MODE || debug_mode || m_mode;

// Effective privilege for trap entry. Used for:
// - Privilege level of trap target fetches (frontend keeps fetch privilege
//   constant during sequential fetch)

assign m_mode_trap_entry = !U_MODE || (
	except == EXCEPT_MRET ? mstatus_mpp : 1'b1
);

// Effective privilege for load/stores. Used for:
// - Privilege level of load/stores on the bus
// - Checking load/stores against PMP read/write permission

assign m_mode_loadstore = !U_MODE || debug_mode || (
	mstatus_mprv ? mstatus_mpp : m_mode
);

// ----------------------------------------------------------------------------
// Properties






































































































endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2023 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

`default_nettype none

module hazard3_decode #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
,
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// These really ought to be localparams, but are occasionally needed for
// passing flags around between modules, so are made available as parameters
// instead. It's ugly, but better scope hygiene than the preprocessor. These
// parameters should not be changed from their default values.

parameter W_REGADDR = 5,

parameter W_ALUOP   = 6,
parameter W_ALUSRC  = 1,
parameter W_MEMOP   = 5,
parameter W_BCOND   = 2,
parameter W_SHAMT   = 5,

parameter W_EXCEPT  = 4,
parameter W_MULOP   = 3
) (
	input  wire                 clk,
	input  wire                 rst_n,

	input  wire [31:0]          fd_cir,
	input  wire [1:0]           fd_cir_err,
	input  wire [1:0]           fd_cir_predbranch,
	input  wire [1:0]           fd_cir_vld,
	input  wire                 fd_cir_is_32bit,
	input  wire                 fd_cir_invalid_16bit,
	input  wire                 fd_cir_is_uop,
	input  wire                 fd_cir_uop_nonfinal,
	input  wire                 fd_cir_uop_no_pc_update,
	input  wire                 fd_cir_uop_atomic,

	output wire [1:0]           df_cir_use,
	output wire                 df_cir_flush_behind,

	output wire                 df_uop_stall,
	output wire                 df_uop_clear,
	output wire                 df_lspair_phase_next,

	output wire [W_ADDR-1:0]    d_pc,

	input  wire                 debug_mode,
	input  wire                 m_mode,
	input  wire                 trap_wfi,

	input  wire [W_ADDR-1:0]    debug_dpc_wdata,
	input  wire                 debug_dpc_wen,
	output wire [W_ADDR-1:0]    debug_dpc_rdata,

	output wire                 d_starved,
	input  wire                 x_stall,
	input  wire                 f_jump_now,
	input  wire [W_ADDR-1:0]    f_jump_target,
	input  wire                 x_jump_not_except,
	input  wire [W_ADDR-1:0]    d_btb_target_addr,

	output reg  [W_DATA-1:0]    d_imm,
	output reg  [W_REGADDR-1:0] d_rs1,
	output reg  [W_REGADDR-1:0] d_rs2,
	output reg  [W_REGADDR-1:0] d_rd,
	output reg  [2:0]           d_funct3_32b,
	output reg  [6:0]           d_funct7_32b,
	output reg  [W_ALUSRC-1:0]  d_alusrc_a,
	output reg  [W_ALUSRC-1:0]  d_alusrc_b,
	output reg  [W_ALUOP-1:0]   d_aluop,
	output reg  [W_MEMOP-1:0]   d_memop,
	output reg  [W_MULOP-1:0]   d_mulop,
	output reg                  d_csr_ren,
	output reg                  d_csr_wen,
	output reg  [1:0]           d_csr_wtype,
	output reg                  d_csr_w_imm,
	output reg  [W_BCOND-1:0]   d_branchcond,
	output reg  [W_ADDR-1:0]    d_addr_offs,
	output reg                  d_addr_is_regoffs,
	output reg  [W_EXCEPT-1:0]  d_except,
	output reg                  d_sleep_wfi,
	output reg                  d_sleep_block,
	output reg                  d_sleep_unblock,
	output wire                 d_no_pc_increment,
	output wire                 d_uninterruptible,
	output wire [W_ADDR-1:0]    d_lspair_offset,
	output reg                  d_fence_i,
	output reg                  d_fence_d
);

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

localparam RV_RS1_LSB = 15;
localparam RV_RS1_BITS = 5;
localparam RV_RS2_LSB = 20;
localparam RV_RS2_BITS = 5;
localparam RV_RD_LSB = 7;
localparam RV_RD_BITS = 5;

// Note: these are preprocessor macros, rather than the usual localparams,
// because it's quite difficult to get a definitive citation from 1364-2005
// for whether Z values are propagated through a localparam to a casez.
// Multiple tools complain about it, so just this once I'll use macros.




































































































































































































































































/*****************************************************************************\
|                        Copyright (C) 2021 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// ALU operation selectors

localparam ALUOP_ADD     = 6'h00; 
localparam ALUOP_SUB     = 6'h01; 
localparam ALUOP_LT      = 6'h02;
localparam ALUOP_LTU     = 6'h04;
localparam ALUOP_AND     = 6'h06;
localparam ALUOP_OR      = 6'h07;
localparam ALUOP_XOR     = 6'h08;
localparam ALUOP_SRL     = 6'h09;
localparam ALUOP_SRA     = 6'h0a;
localparam ALUOP_SLL     = 6'h0b;
localparam ALUOP_MULDIV  = 6'h0c;
localparam ALUOP_RS2     = 6'h0d; // differs from AND/OR/XOR in [1:0]
// Bitmanip ALU operations (some also used by AMOs):
localparam ALUOP_SHXADD  = 6'h20;
localparam ALUOP_CLZ     = 6'h23;
localparam ALUOP_CPOP    = 6'h24;
localparam ALUOP_CTZ     = 6'h25;
localparam ALUOP_ANDN    = 6'h26; // Same LSBs as non-inverted
localparam ALUOP_ORN     = 6'h27; // Same LSBs as non-inverted
localparam ALUOP_XNOR    = 6'h28; // Same LSBs as non-inverted
localparam ALUOP_MAX     = 6'h29;
localparam ALUOP_MAXU    = 6'h2a;
localparam ALUOP_MIN     = 6'h2b;
localparam ALUOP_MINU    = 6'h2c;
localparam ALUOP_ORC_B   = 6'h2d;
localparam ALUOP_REV8    = 6'h2e;
localparam ALUOP_ROL     = 6'h2f;
localparam ALUOP_ROR     = 6'h30;
localparam ALUOP_SEXT_B  = 6'h31;
localparam ALUOP_SEXT_H  = 6'h32;
localparam ALUOP_ZEXT_H  = 6'h33;

localparam ALUOP_CLMUL   = 6'h34;

localparam ALUOP_BCLR    = 6'h35;
localparam ALUOP_BEXT    = 6'h36;
localparam ALUOP_BINV    = 6'h37;
localparam ALUOP_BSET    = 6'h38;

localparam ALUOP_PACK    = 6'h39;
localparam ALUOP_PACKH   = 6'h3a;
localparam ALUOP_BREV8   = 6'h3b;
localparam ALUOP_ZIP     = 6'h3c;
localparam ALUOP_UNZIP   = 6'h3d;

localparam ALUOP_BEXTM   = 6'h3e;

localparam ALUOP_XPERM   = 6'h3f;

// Parameters to control ALU input muxes. Bypass mux paths are
// controlled by X, so D has no parameters to choose these.

localparam ALUSRCA_RS1 = 1'h0;
localparam ALUSRCA_PC  = 1'h1;

localparam ALUSRCB_RS2 = 1'h0;
localparam ALUSRCB_IMM = 1'h1;

localparam MEMOP_LW   = 5'h00;
localparam MEMOP_LH   = 5'h01;
localparam MEMOP_LB   = 5'h02;
localparam MEMOP_LHU  = 5'h03;
localparam MEMOP_LBU  = 5'h04;
localparam MEMOP_SW   = 5'h05;
localparam MEMOP_SH   = 5'h06;
localparam MEMOP_SB   = 5'h07;

localparam MEMOP_LR_W = 5'h08;
localparam MEMOP_SC_W = 5'h09;
localparam MEMOP_AMO  = 5'h0a;
localparam MEMOP_NONE = 5'h10;

localparam BCOND_NEVER  = 2'h0;
localparam BCOND_ALWAYS = 2'h1;
localparam BCOND_ZERO   = 2'h2;
localparam BCOND_NZERO  = 2'h3;

// CSR access types

localparam CSR_WTYPE_W    = 2'h0;
localparam CSR_WTYPE_S    = 2'h1;
localparam CSR_WTYPE_C    = 2'h2;

// Exceptional condition signals which travel alongside (or instead of)
// instructions in the pipeline. These are speculative and can be flushed on
// e.g. branch mispredict. These mostly align with mcause values.

localparam EXCEPT_NONE           = 4'hf;

localparam EXCEPT_INSTR_MISALIGN = 4'h0;
localparam EXCEPT_INSTR_FAULT    = 4'h1;
localparam EXCEPT_INSTR_ILLEGAL  = 4'h2;
localparam EXCEPT_EBREAK         = 4'h3;
localparam EXCEPT_LOAD_ALIGN     = 4'h4;
localparam EXCEPT_LOAD_FAULT     = 4'h5;
localparam EXCEPT_STORE_ALIGN    = 4'h6;
localparam EXCEPT_STORE_FAULT    = 4'h7;
localparam EXCEPT_ECALL_U        = 4'h8;
// MRET, Return from M-mode: not really an exception, but handled like one
localparam EXCEPT_MRET           = 4'ha;
localparam EXCEPT_ECALL_M        = 4'hb;
// spare: c
// spare: d
// REFETCH: flush and refetch sequentially-following instructions, e.g. on
// executing fence.i. Jumps from stage 3 to get ordering against L/S dphase.
localparam EXCEPT_REFETCH        = 4'he;

// Operations for M extension (these are just instr[14:12])

localparam M_OP_MUL    = 3'h0;
localparam M_OP_MULH   = 3'h1;
localparam M_OP_MULHSU = 3'h2;
localparam M_OP_MULHU  = 3'h3;
localparam M_OP_DIV    = 3'h4;
localparam M_OP_DIVU   = 3'h5;
localparam M_OP_REM    = 3'h6;
localparam M_OP_REMU   = 3'h7;

localparam HAVE_CSR = CSR_M_MANDATORY || CSR_M_TRAP || CSR_COUNTER;

// ----------------------------------------------------------------------------

wire [31:0] d_instr = fd_cir | {
	30'd0, {2{~|EXTENSION_C}}
};

reg  d_invalid_32bit;
wire d_invalid = fd_cir_invalid_16bit || d_invalid_32bit;

assign d_uninterruptible = |EXTENSION_ZCMP && fd_cir_uop_atomic;








wire d_lspair_nonfinal;
// Signal to null the mepc offset when taking an exception on this
// instruction (because uops in a sequence *which can except*, so excluding
// the final sp adjust on popret/popretz, will all have the same PC as the
// next uop, which will be in stage 2 when they take their exception)
assign d_no_pc_increment = fd_cir_uop_nonfinal || d_lspair_nonfinal;

assign df_uop_stall = x_stall || d_starved;

// Note !df_cir_flush_behind because the jump in cm.popret/popretz is the
// *penultimate* instruction: we execute the stack adjustment in the fetch
// bubble to save a cycle, still need to finish the uop sequence.
//
// The sp adjust cannot generate an exception (it's an `add` with the same
// PMP.X and breakpoint comparison results as earlier uops) and interrupts are
// suppressed for this part of the sequence.
assign df_uop_clear = f_jump_now && !df_cir_flush_behind;

// Decode various immediate formats
wire [31:0] d_imm_i = {{21{d_instr[31]}}, d_instr[30:20]};
wire [31:0] d_imm_s = {{21{d_instr[31]}}, d_instr[30:25], d_instr[11:7]};
wire [31:0] d_imm_b = {{20{d_instr[31]}}, d_instr[7], d_instr[30:25], d_instr[11:8], 1'b0};
wire [31:0] d_imm_u = {d_instr[31:12], {12{1'b0}}};
wire [31:0] d_imm_j = {{12{d_instr[31]}}, d_instr[19:12], d_instr[20], d_instr[30:21], 1'b0};

// ----------------------------------------------------------------------------
// PC/CIR control

// Must not flag bus error for a valid 16-bit instruction *followed by* an
// error, because instruction fetch errors are speculative, and can be
// flushed by e.g. a branch instruction. Note the 16 LSBs must be valid for
// us to know an instruction's size.
wire d_except_instr_bus_fault = fd_cir_vld > 2'd0 && fd_cir_err[0] ||
	fd_cir_vld > 2'd1 && fd_cir_is_32bit && fd_cir_err[1];

assign d_starved = ~|fd_cir_vld || fd_cir_vld[0] && fd_cir_is_32bit;

wire d_stall = x_stall || d_starved || fd_cir_uop_nonfinal || d_lspair_nonfinal;

assign df_cir_use =
	d_starved || d_stall ? 2'h0 :
	fd_cir_is_32bit ? 2'h2 : 2'h1;

// CIR Locking is required if we successfully assert a jump request, but
// decode is stalled. It is not possible to gate the jump request if the
// stall depends on bus stall (as this would create a through-path from bus
// stall to bus request) so instead we instruct the frontend to preserve the
// stalled instruction when flushing, and fill in behind it.
//
// Once the stall clears, the stalled instruction can execute its remaining
// side effects e.g. writing a link value to the register file.
wire jump_caused_by_d = f_jump_now && x_jump_not_except;
wire assert_cir_lock = jump_caused_by_d && d_stall;

// CIR lock ends naturally when an instruction (not just uop) graduates to the
// next stage:
wire finished_cir_lock = !d_stall;

// CIR lock can meet an untimely end due to trap entry. One way to reach this
// is a dphase load fault on the final load in a cm.popret: here the `ret`
// issues a fetch address while stalled on the first dphase cycle, then is
// flushed by trap on second cycle.
wire deassert_cir_lock = finished_cir_lock || (f_jump_now && !x_jump_not_except);

reg cir_lock_prev;
wire cir_lock = (cir_lock_prev && !deassert_cir_lock) || assert_cir_lock;
assign df_cir_flush_behind = assert_cir_lock && !cir_lock_prev;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		cir_lock_prev <= 1'b0;
	end else begin
		cir_lock_prev <= cir_lock;
	end
end

reg  [W_ADDR-1:0] pc;
wire [W_ADDR-1:0] pc_seq_next = pc + (
	|EXTENSION_ZCMP && fd_cir_is_uop && fd_cir_uop_no_pc_update ? 32'd0 :
	fd_cir_is_32bit                                             ? 32'd4 : 32'd2
);

assign d_pc = pc;
assign debug_dpc_rdata = pc;

// Frontend should mark the whole instruction, and nothing but the
// instruction, as a predicted branch. This goes wrong when we execute the
// address containing the predicted branch twice with different 16-bit
// alignments (!). We need to issue a branch-to-self to get back on a linear
// path, otherwise PC and CIR will diverge and we will misexecute.
wire partial_predicted_branch = !d_starved &&
	|BRANCH_PREDICTOR && fd_cir_is_32bit && ^fd_cir_predbranch;

wire predicted_branch = |BRANCH_PREDICTOR && fd_cir_predbranch[0];

// Generally locking takes place on a stalled jump/branch, which may need the
// original PC available to produce a link address when it unstalls. An
// exception to this is jumps in micro-op sequences: in this case the jump is
// the penultimate instruction in the sequence (ret before addi sp) and we
// need to capture the pc mid-uop-sequence.
wire hold_pc_on_cir_lock = assert_cir_lock && !(fd_cir_is_uop && !fd_cir_uop_no_pc_update && !x_stall);
wire update_pc_on_cir_unlock = cir_lock_prev && finished_cir_lock && !fd_cir_uop_no_pc_update;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		pc <= RESET_VECTOR;
	end else begin
		if (debug_dpc_wen) begin
			pc <= debug_dpc_wdata;
		end else if (debug_mode) begin
			pc <= pc;
		end else if ((f_jump_now && !hold_pc_on_cir_lock) || update_pc_on_cir_unlock) begin
			pc <= f_jump_target;
		end else if (!f_jump_now && fd_cir_uop_nonfinal && !fd_cir_uop_no_pc_update && !x_stall) begin
			// End of previously stalled jr uop in cm.popret and cm.popretz:
			// safe to update PC as next instruction (addi sp) cannot trap.
			pc <= f_jump_target;
		end else if (!d_stall && !cir_lock) begin
			// If this instruction is a predicted-taken branch (and has not
			// generated a mispredict recovery jump) then set PC to the
			// prediction target instead of the sequentially next PC
			pc <= predicted_branch ? d_btb_target_addr : pc_seq_next;
		end
	end
end













wire [W_ADDR-1:0] branch_offs =
	!fd_cir_is_32bit && predicted_branch ? 32'd2 :
	 fd_cir_is_32bit && predicted_branch ? 32'd4 : d_imm_b;

always @ (*) begin
	casez ({|EXTENSION_A, d_instr[6:2]})
	{1'bz, 5'b11011}: d_addr_offs = d_imm_j      ; // JAL
	{1'bz, 5'b11000}: d_addr_offs = branch_offs  ; // Branches
	{1'bz, 5'b01000}: d_addr_offs = d_imm_s      ; // Store
	{1'bz, 5'b11001}: d_addr_offs = d_imm_i      ; // JALR
	{1'bz, 5'b00000}: d_addr_offs = d_imm_i      ; // Loads
	{1'b1, 5'b01011}: d_addr_offs = 32'h0000_0000; // Atomics
	default:          d_addr_offs = 32'hxxxx_xxxx;
	endcase
	if (partial_predicted_branch) begin
		d_addr_offs = 32'h0000_0000;
	end
end

// ----------------------------------------------------------------------------
// Track phase of load/store pair instructions (Zilsd and Zclsd)

// This could be shared with uop_ctr (for Zcmp) but the two are fundamentally
// different: Zcmp has 16-bit instructions which expand to sequences of
// 32-bit, whereas Zilsd has multi-phase 32-bit instructions and Zclsd has
// direct 16-bit aliases of those instructions. Therefore it's cleaner to
// separate the phasing from the decompression for Zilsd/Zclsd.

wire d_lspair_phase;

// Reorder accesses to avoid clobbering rs1 (base) in first half of load:
wire d_lspair_reg_sel = d_lspair_phase == d_instr[15];

generate
if (EXTENSION_ZILSD) begin: have_lspair_reg_sel
	reg d_lspair_phase_r;
	assign d_lspair_phase = d_lspair_phase_r;
	reg instr_is_lspair;
	always @ (*) begin
		casez ({d_invalid || d_starved, d_instr})
			{1'b0, 32'b?????????????????011????00000011}: instr_is_lspair = 1'b1;
			{1'b0, 32'b???????????0?????011?????0100011}: instr_is_lspair = 1'b1;
			default:           instr_is_lspair = 1'b0;
		endcase
	end

	always @ (posedge clk or negedge rst_n) begin
		if (!rst_n) begin
			d_lspair_phase_r <= 1'b0;
		end else begin
			d_lspair_phase_r <= df_lspair_phase_next;
		end
	end

	assign df_lspair_phase_next =
		!d_stall || f_jump_now      ? 1'b0 :
		instr_is_lspair && !x_stall ? 1'b1 : d_lspair_phase_r;

	assign d_lspair_nonfinal = instr_is_lspair && !d_lspair_phase_r;

	assign d_lspair_offset = {
		29'h0,
		d_lspair_reg_sel && instr_is_lspair,
		2'h0
	};

end else begin: no_lspair_reg_sel

	assign d_lspair_phase = 1'b0;
	assign df_lspair_phase_next = 1'b0;
	assign d_lspair_nonfinal = 1'b0;
	assign d_lspair_offset = 32'd0;

end
endgenerate

// ----------------------------------------------------------------------------
// Decode X controls

localparam X0 = {W_REGADDR{1'b0}};

// First decode the instruction bits, based on available extensions and
// privilege state, without gating in any stall/exception signals.
reg  [W_REGADDR-1:0] raw_rs1;
reg  [W_REGADDR-1:0] raw_rs2;
reg  [W_REGADDR-1:0] raw_rd;
reg  [W_DATA-1:0]    raw_imm;
reg  [W_ALUSRC-1:0]  raw_alusrc_a;
reg  [W_ALUSRC-1:0]  raw_alusrc_b;
reg  [W_ALUOP-1:0]   raw_aluop;
reg  [W_MEMOP-1:0]   raw_memop;
reg  [W_MULOP-1:0]   raw_mulop;
reg                  raw_csr_ren;
reg                  raw_csr_wen;
reg  [1:0]           raw_csr_wtype;
reg                  raw_csr_w_imm;
reg  [W_BCOND-1:0]   raw_branchcond;
reg                  raw_addr_is_regoffs;
reg  [W_EXCEPT-1:0]  raw_except;
reg                  raw_sleep_wfi;
reg                  raw_sleep_block;
reg                  raw_sleep_unblock;
reg                  raw_fence_i;
reg                  raw_fence_d;

always @ (*) begin
	// Assign some defaults
	raw_rs1 = d_instr[19:15];
	raw_rs2 = d_instr[24:20];
	raw_rd  = d_instr[11: 7];
	raw_imm = d_imm_i;
	raw_alusrc_a = ALUSRCA_RS1;
	raw_alusrc_b = ALUSRCB_RS2;
	raw_aluop = ALUOP_ADD;
	raw_memop = MEMOP_NONE;
	raw_mulop = M_OP_MUL;
	raw_csr_ren = 1'b0;
	raw_csr_wen = 1'b0;
	raw_csr_wtype = CSR_WTYPE_W;
	raw_csr_w_imm = 1'b0;
	raw_branchcond = BCOND_NEVER;
	raw_addr_is_regoffs = 1'b0;
	raw_except = EXCEPT_NONE;
	raw_sleep_wfi = 1'b0;
	raw_sleep_block = 1'b0;
	raw_sleep_unblock = 1'b0;
	raw_fence_i = 1'b0;
	raw_fence_d = 1'b0;
	// Note this funct3/funct7 are valid only for 32-bit instructions. They
	// are useful for clusters of related ALU ops, such as sh*add, clmul.
	d_funct3_32b = fd_cir[14:12];
	d_funct7_32b = fd_cir[31:25];

	d_invalid_32bit = 1'b0;

	casez (d_instr)
	32'b?????????????????000?????1100011:       begin d_invalid_32bit = DEBUG_SUPPORT && debug_mode; raw_rd = X0; raw_aluop = ALUOP_SUB; raw_branchcond = BCOND_ZERO;  end
	32'b?????????????????001?????1100011:       begin d_invalid_32bit = DEBUG_SUPPORT && debug_mode; raw_rd = X0; raw_aluop = ALUOP_SUB; raw_branchcond = BCOND_NZERO; end
	32'b?????????????????100?????1100011:       begin d_invalid_32bit = DEBUG_SUPPORT && debug_mode; raw_rd = X0; raw_aluop = ALUOP_LT;  raw_branchcond = BCOND_NZERO; end
	32'b?????????????????101?????1100011:       begin d_invalid_32bit = DEBUG_SUPPORT && debug_mode; raw_rd = X0; raw_aluop = ALUOP_LT;  raw_branchcond = BCOND_ZERO;  end
	32'b?????????????????110?????1100011:      begin d_invalid_32bit = DEBUG_SUPPORT && debug_mode; raw_rd = X0; raw_aluop = ALUOP_LTU; raw_branchcond = BCOND_NZERO; end
	32'b?????????????????111?????1100011:      begin d_invalid_32bit = DEBUG_SUPPORT && debug_mode; raw_rd = X0; raw_aluop = ALUOP_LTU; raw_branchcond = BCOND_ZERO;  end
	32'b?????????????????000?????1100111:      begin d_invalid_32bit = DEBUG_SUPPORT && debug_mode; raw_branchcond = BCOND_ALWAYS; raw_addr_is_regoffs = 1'b1;
	                        raw_rs2 = X0; raw_aluop = ALUOP_ADD; raw_alusrc_a = ALUSRCA_PC; raw_alusrc_b = ALUSRCB_IMM; raw_imm = fd_cir_is_32bit ? 32'd4 : 32'd2; end
	32'b?????????????????????????1101111:       begin d_invalid_32bit = DEBUG_SUPPORT && debug_mode; raw_branchcond = BCOND_ALWAYS; raw_rs1 = X0;
	                        raw_rs2 = X0; raw_aluop = ALUOP_ADD; raw_alusrc_a = ALUSRCA_PC; raw_alusrc_b = ALUSRCB_IMM; raw_imm = fd_cir_is_32bit ? 32'd4 : 32'd2; end
	32'b?????????????????????????0110111:       begin raw_aluop = ALUOP_RS2; raw_imm = d_imm_u; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; raw_rs1 = X0; end
	32'b?????????????????????????0010111:     begin d_invalid_32bit = DEBUG_SUPPORT && debug_mode; raw_aluop = ALUOP_ADD; raw_imm = d_imm_u; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; raw_alusrc_a = ALUSRCA_PC;  raw_rs1 = X0; end
	32'b?????????????????000?????0010011:      begin raw_aluop = ALUOP_ADD; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; end
	32'b0000000??????????001?????0010011:      begin raw_aluop = ALUOP_SLL; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; end
	32'b?????????????????010?????0010011:      begin raw_aluop = ALUOP_LT;  raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; end
	32'b?????????????????011?????0010011:     begin raw_aluop = ALUOP_LTU; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; end
	32'b?????????????????100?????0010011:      begin raw_aluop = ALUOP_XOR; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; end
	32'b0000000??????????101?????0010011:      begin raw_aluop = ALUOP_SRL; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; end
	32'b0100000??????????101?????0010011:      begin raw_aluop = ALUOP_SRA; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; end
	32'b?????????????????110?????0010011:       begin raw_aluop = ALUOP_OR;  raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; end
	32'b?????????????????111?????0010011:      begin raw_aluop = ALUOP_AND; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; raw_rs2 = X0; end
	32'b0000000??????????000?????0110011:       begin raw_aluop = ALUOP_ADD; end
	32'b0100000??????????000?????0110011:       begin raw_aluop = ALUOP_SUB; end
	32'b0000000??????????001?????0110011:       begin raw_aluop = ALUOP_SLL; end
	32'b0000000??????????011?????0110011:      begin raw_aluop = ALUOP_LTU; end
	32'b0000000??????????100?????0110011:       begin raw_aluop = ALUOP_XOR; end
	32'b0000000??????????101?????0110011:       begin raw_aluop = ALUOP_SRL; end
	32'b0100000??????????101?????0110011:       begin raw_aluop = ALUOP_SRA; end
	32'b0000000??????????110?????0110011:        begin raw_aluop = ALUOP_OR;  end
	32'b0000000??????????111?????0110011:       begin raw_aluop = ALUOP_AND; end
	32'b?????????????????000?????0000011:        begin raw_addr_is_regoffs = 1'b1; raw_rs2 = X0; raw_memop = MEMOP_LB;  end
	32'b?????????????????001?????0000011:        begin raw_addr_is_regoffs = 1'b1; raw_rs2 = X0; raw_memop = MEMOP_LH;  end
	32'b?????????????????010?????0000011:        begin raw_addr_is_regoffs = 1'b1; raw_rs2 = X0; raw_memop = MEMOP_LW;  end
	32'b?????????????????100?????0000011:       begin raw_addr_is_regoffs = 1'b1; raw_rs2 = X0; raw_memop = MEMOP_LBU; end
	32'b?????????????????101?????0000011:       begin raw_addr_is_regoffs = 1'b1; raw_rs2 = X0; raw_memop = MEMOP_LHU; end
	32'b?????????????????000?????0100011:        begin raw_addr_is_regoffs = 1'b1; raw_aluop = ALUOP_RS2; raw_memop = MEMOP_SB;  raw_rd = X0; end
	32'b?????????????????001?????0100011:        begin raw_addr_is_regoffs = 1'b1; raw_aluop = ALUOP_RS2; raw_memop = MEMOP_SH;  raw_rd = X0; end
	32'b?????????????????010?????0100011:        begin raw_addr_is_regoffs = 1'b1; raw_aluop = ALUOP_RS2; raw_memop = MEMOP_SW;  raw_rd = X0; end

	32'b0000000??????????010?????0110011:       begin
		raw_aluop = ALUOP_LT;
		if (|EXTENSION_XH3POWER && ~|raw_rd && ~|raw_rs1) begin
			if (raw_rs2 == 5'h00) begin
				// h3.block (power management hint)
				d_invalid_32bit = trap_wfi;
				raw_sleep_block = !trap_wfi;
			end else if (raw_rs2 == 5'h01) begin
				// h3.unblock (power management hint)
				d_invalid_32bit = trap_wfi;
				raw_sleep_unblock = !trap_wfi;
			end
		end
	end

	32'b0000001??????????000?????0110011:       if (EXTENSION_M) begin raw_aluop = ALUOP_MULDIV; raw_mulop = M_OP_MUL;    end else begin d_invalid_32bit = 1'b1; end
	32'b0000001??????????001?????0110011:      if (EXTENSION_M) begin raw_aluop = ALUOP_MULDIV; raw_mulop = M_OP_MULH;   end else begin d_invalid_32bit = 1'b1; end
	32'b0000001??????????010?????0110011:    if (EXTENSION_M) begin raw_aluop = ALUOP_MULDIV; raw_mulop = M_OP_MULHSU; end else begin d_invalid_32bit = 1'b1; end
	32'b0000001??????????011?????0110011:     if (EXTENSION_M) begin raw_aluop = ALUOP_MULDIV; raw_mulop = M_OP_MULHU;  end else begin d_invalid_32bit = 1'b1; end
	32'b0000001??????????100?????0110011:       if (EXTENSION_M) begin raw_aluop = ALUOP_MULDIV; raw_mulop = M_OP_DIV;    end else begin d_invalid_32bit = 1'b1; end
	32'b0000001??????????101?????0110011:      if (EXTENSION_M) begin raw_aluop = ALUOP_MULDIV; raw_mulop = M_OP_DIVU;   end else begin d_invalid_32bit = 1'b1; end
	32'b0000001??????????110?????0110011:       if (EXTENSION_M) begin raw_aluop = ALUOP_MULDIV; raw_mulop = M_OP_REM;    end else begin d_invalid_32bit = 1'b1; end
	32'b0000001??????????111?????0110011:      if (EXTENSION_M) begin raw_aluop = ALUOP_MULDIV; raw_mulop = M_OP_REMU;   end else begin d_invalid_32bit = 1'b1; end

	32'b00010??00000?????010?????0101111:      if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_LR_W; raw_rs2 = X0;           end else begin d_invalid_32bit = 1'b1; end
	32'b00011????????????010?????0101111:      if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_SC_W; raw_aluop = ALUOP_RS2;  end else begin d_invalid_32bit = 1'b1; end
	32'b00001????????????010?????0101111: if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_AMO;  raw_aluop = ALUOP_RS2;  end else begin d_invalid_32bit = 1'b1; end
	32'b00000????????????010?????0101111:  if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_AMO;  raw_aluop = ALUOP_ADD;  end else begin d_invalid_32bit = 1'b1; end
	32'b00100????????????010?????0101111:  if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_AMO;  raw_aluop = ALUOP_XOR;  end else begin d_invalid_32bit = 1'b1; end
	32'b01100????????????010?????0101111:  if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_AMO;  raw_aluop = ALUOP_AND;  end else begin d_invalid_32bit = 1'b1; end
	32'b01000????????????010?????0101111:   if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_AMO;  raw_aluop = ALUOP_OR;   end else begin d_invalid_32bit = 1'b1; end
	32'b10000????????????010?????0101111:  if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_AMO;  raw_aluop = ALUOP_MIN;  end else begin d_invalid_32bit = 1'b1; end
	32'b10100????????????010?????0101111:  if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_AMO;  raw_aluop = ALUOP_MAX;  end else begin d_invalid_32bit = 1'b1; end
	32'b11000????????????010?????0101111: if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_AMO;  raw_aluop = ALUOP_MINU; end else begin d_invalid_32bit = 1'b1; end
	32'b11100????????????010?????0101111: if (EXTENSION_A) begin raw_addr_is_regoffs = 1'b1; raw_memop = MEMOP_AMO;  raw_aluop = ALUOP_MAXU; end else begin d_invalid_32bit = 1'b1; end

	32'b?????????????????011????00000011:        if (EXTENSION_ZILSD) begin raw_addr_is_regoffs = 1'b1; raw_rs2 = X0; raw_memop = MEMOP_LW; raw_rd  = {d_instr[11: 8], d_lspair_reg_sel};                        end else begin d_invalid_32bit = 1'b1; end
	32'b???????????0?????011?????0100011:        if (EXTENSION_ZILSD) begin raw_addr_is_regoffs = 1'b1; raw_rd  = X0; raw_memop = MEMOP_SW; raw_rs2 = {d_instr[24:21], d_lspair_reg_sel}; raw_aluop = ALUOP_RS2; end else begin d_invalid_32bit = 1'b1; end

	32'b0010000??????????010?????0110011:    if (EXTENSION_ZBA)  begin raw_aluop = ALUOP_SHXADD;                                                              end else begin d_invalid_32bit = 1'b1; end
	32'b0010000??????????100?????0110011:    if (EXTENSION_ZBA)  begin raw_aluop = ALUOP_SHXADD;                                                              end else begin d_invalid_32bit = 1'b1; end
	32'b0010000??????????110?????0110011:    if (EXTENSION_ZBA)  begin raw_aluop = ALUOP_SHXADD;                                                              end else begin d_invalid_32bit = 1'b1; end

	32'b0100000??????????111?????0110011:      if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_ANDN;                                                                end else begin d_invalid_32bit = 1'b1; end
	32'b011000000000?????001?????0010011:       if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_CLZ;    raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end
	32'b011000000010?????001?????0010011:      if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_CPOP;   raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end
	32'b011000000001?????001?????0010011:       if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_CTZ;    raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0000101??????????110?????0110011:       if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_MAX;                                                                 end else begin d_invalid_32bit = 1'b1; end
	32'b0000101??????????111?????0110011:      if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_MAXU;                                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0000101??????????100?????0110011:       if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_MIN;                                                                 end else begin d_invalid_32bit = 1'b1; end
	32'b0000101??????????101?????0110011:      if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_MINU;                                                                end else begin d_invalid_32bit = 1'b1; end
	32'b001010000111?????101?????0010011:     if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_ORC_B;  raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0100000??????????110?????0110011:       if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_ORN;                                                                 end else begin d_invalid_32bit = 1'b1; end
	32'b011010011000?????101?????0010011:      if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_REV8;   raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0110000??????????001?????0110011:       if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_ROL;                                                                 end else begin d_invalid_32bit = 1'b1; end
	32'b0110000??????????101?????0110011:       if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_ROR;                                                                 end else begin d_invalid_32bit = 1'b1; end
	32'b0110000??????????101?????0010011:      if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_ROR;    raw_rs2 = X0; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; end else begin d_invalid_32bit = 1'b1; end
	32'b011000000100?????001?????0010011:    if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_SEXT_B; raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end
	32'b011000000101?????001?????0010011:    if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_SEXT_H; raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0100000??????????100?????0110011:      if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_XNOR;                                                                end else begin d_invalid_32bit = 1'b1; end
	// Note: ZEXT_H is a subset of PACK from Zbkb. This is fine as long
	// as this case appears first, since Zbkb implies Zbb on Hazard3.
	32'b000010000000?????100?????0110011:    if (EXTENSION_ZBB)  begin raw_aluop = ALUOP_ZEXT_H; raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end

	32'b0000101??????????001?????0110011:     if (EXTENSION_ZBC)  begin raw_aluop = ALUOP_CLMUL;                                                               end else begin d_invalid_32bit = 1'b1; end
	32'b0000101??????????011?????0110011:    if (EXTENSION_ZBC)  begin raw_aluop = ALUOP_CLMUL;                                                               end else begin d_invalid_32bit = 1'b1; end
	32'b0000101??????????010?????0110011:    if (EXTENSION_ZBC)  begin raw_aluop = ALUOP_CLMUL;                                                               end else begin d_invalid_32bit = 1'b1; end

	32'b0100100??????????001?????0110011:      if (EXTENSION_ZBS)  begin raw_aluop = ALUOP_BCLR;                                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0100100??????????001?????0010011:     if (EXTENSION_ZBS)  begin raw_aluop = ALUOP_BCLR;   raw_rs2 = X0; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; end else begin d_invalid_32bit = 1'b1; end
	32'b0100100??????????101?????0110011:      if (EXTENSION_ZBS)  begin raw_aluop = ALUOP_BEXT;                                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0100100??????????101?????0010011:     if (EXTENSION_ZBS)  begin raw_aluop = ALUOP_BEXT;   raw_rs2 = X0; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; end else begin d_invalid_32bit = 1'b1; end
	32'b0110100??????????001?????0110011:      if (EXTENSION_ZBS)  begin raw_aluop = ALUOP_BINV;                                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0110100??????????001?????0010011:     if (EXTENSION_ZBS)  begin raw_aluop = ALUOP_BINV;   raw_rs2 = X0; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; end else begin d_invalid_32bit = 1'b1; end
	32'b0010100??????????001?????0110011:      if (EXTENSION_ZBS)  begin raw_aluop = ALUOP_BSET;                                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0010100??????????001?????0010011:     if (EXTENSION_ZBS)  begin raw_aluop = ALUOP_BSET;   raw_rs2 = X0; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; end else begin d_invalid_32bit = 1'b1; end

	32'b0000100??????????100?????0110011:      if (EXTENSION_ZBKB) begin raw_aluop = ALUOP_PACK;                                                                end else begin d_invalid_32bit = 1'b1; end
	32'b0000100??????????111?????0110011:     if (EXTENSION_ZBKB) begin raw_aluop = ALUOP_PACKH;                                                               end else begin d_invalid_32bit = 1'b1; end
	32'b011010000111?????101?????0010011:     if (EXTENSION_ZBKB) begin raw_aluop = ALUOP_BREV8;  raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end
	32'b000010001111?????101?????0010011:     if (EXTENSION_ZBKB) begin raw_aluop = ALUOP_UNZIP;  raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end
	32'b000010001111?????001?????0010011:       if (EXTENSION_ZBKB) begin raw_aluop = ALUOP_ZIP;    raw_rs2 = X0;                                                end else begin d_invalid_32bit = 1'b1; end

	32'b0010100??????????100?????0110011:    if (EXTENSION_ZBKX) begin raw_aluop = ALUOP_XPERM;                                                               end else begin d_invalid_32bit = 1'b1; end
	32'b0010100??????????010?????0110011:    if (EXTENSION_ZBKX) begin raw_aluop = ALUOP_XPERM;                                                               end else begin d_invalid_32bit = 1'b1; end

	32'b000???0??????????000?????0001011:  if (EXTENSION_XH3BEXTM) begin
	                                           raw_aluop = ALUOP_BEXTM;                                                                end else begin d_invalid_32bit = 1'b1; end
	32'b000???0??????????100?????0001011: if (EXTENSION_XH3BEXTM) begin
	                                           raw_aluop = ALUOP_BEXTM;   raw_rs2 = X0; raw_imm = d_imm_i; raw_alusrc_b = ALUSRCB_IMM; end else begin d_invalid_32bit = 1'b1; end
	32'b?????????????????001?????1110011:     if (HAVE_CSR)              begin raw_rs2 = X0; raw_imm = d_imm_i; raw_csr_wen = 1'b1  ;   raw_csr_ren = |raw_rd; raw_csr_wtype = CSR_WTYPE_W;                       end else begin d_invalid_32bit = 1'b1; end
	32'b?????????????????010?????1110011:     if (HAVE_CSR)              begin raw_rs2 = X0; raw_imm = d_imm_i; raw_csr_wen = |raw_rs1; raw_csr_ren = 1'b1 ;   raw_csr_wtype = CSR_WTYPE_S;                       end else begin d_invalid_32bit = 1'b1; end
	32'b?????????????????011?????1110011:     if (HAVE_CSR)              begin raw_rs2 = X0; raw_imm = d_imm_i; raw_csr_wen = |raw_rs1; raw_csr_ren = 1'b1 ;   raw_csr_wtype = CSR_WTYPE_C;                       end else begin d_invalid_32bit = 1'b1; end
	32'b?????????????????101?????1110011:    if (HAVE_CSR)              begin raw_rs2 = X0; raw_imm = d_imm_i; raw_csr_wen = 1'b1  ;   raw_csr_ren = |raw_rd; raw_csr_wtype = CSR_WTYPE_W; raw_csr_w_imm = 1'b1; end else begin d_invalid_32bit = 1'b1; end
	32'b?????????????????110?????1110011:    if (HAVE_CSR)              begin raw_rs2 = X0; raw_imm = d_imm_i; raw_csr_wen = |raw_rs1; raw_csr_ren = 1'b1 ;   raw_csr_wtype = CSR_WTYPE_S; raw_csr_w_imm = 1'b1; end else begin d_invalid_32bit = 1'b1; end
	32'b?????????????????111?????1110011:    if (HAVE_CSR)              begin raw_rs2 = X0; raw_imm = d_imm_i; raw_csr_wen = |raw_rs1; raw_csr_ren = 1'b1 ;   raw_csr_wtype = CSR_WTYPE_C; raw_csr_w_imm = 1'b1; end else begin d_invalid_32bit = 1'b1; end

	32'b????????????00000000000000001111:     begin raw_rs2 = X0; raw_fence_d = 1'b1; end  // Note rs1/rd are zero in instruction
	32'b00000000000000000001000000001111:   if (EXTENSION_ZIFENCEI)    begin raw_except = debug_mode ? EXCEPT_NONE : EXCEPT_REFETCH; raw_fence_i = 1'b1;                                          end else begin d_invalid_32bit = 1'b1; end // note rs1/rs2/rd are zero in instruction
	32'b00000000000000000000000001110011:     if (HAVE_CSR)              begin raw_except = m_mode || !U_MODE ? EXCEPT_ECALL_M : EXCEPT_ECALL_U;  raw_rs2 = X0; raw_rs1 = X0; raw_rd = X0;          end else begin d_invalid_32bit = 1'b1; end
	32'b00000000000100000000000001110011:    if (HAVE_CSR)              begin raw_except = EXCEPT_EBREAK; raw_rs2 = X0; raw_rs1 = X0; raw_rd = X0;                                                 end else begin d_invalid_32bit = 1'b1; end
	32'b00110000001000000000000001110011:      if (HAVE_CSR && m_mode)    begin raw_except = EXCEPT_MRET;   raw_rs2 = X0; raw_rs1 = X0; raw_rd = X0;                                                 end else begin d_invalid_32bit = 1'b1; end
	32'b00010000010100000000000001110011:       if (HAVE_CSR && !trap_wfi) begin raw_sleep_wfi = 1'b1;       raw_rs2 = X0; raw_rs1 = X0; raw_rd = X0;                                                 end else begin d_invalid_32bit = 1'b1; end

	default:          begin d_invalid_32bit = 1'b1; end
	endcase

	if (|EXTENSION_E && (raw_rd[4] || raw_rs1[4] || raw_rs2[4])) begin
		d_invalid_32bit = 1'b1;
	end
end

// Then gate key signals based on CIR fullness, fetch faults etc. The split
// helps to avoid an event scheduling feedback loop that makes simulators
// unhappy and slow, particularly verilator

localparam [4:0] REGADDR_MASK = {~|EXTENSION_E, 4'hf};

always @ (*) begin
	// Pass through by default
	d_rs1             = raw_rs1 & REGADDR_MASK;
	d_rs2             = raw_rs2 & REGADDR_MASK;
	d_rd              = raw_rd  & REGADDR_MASK;
	d_imm             = raw_imm;
	d_alusrc_a        = raw_alusrc_a;
	d_alusrc_b        = raw_alusrc_b;
	d_aluop           = raw_aluop;
	d_memop           = raw_memop;
	d_mulop           = raw_mulop;
	d_csr_ren         = raw_csr_ren;
	d_csr_wen         = raw_csr_wen;
	d_csr_wtype       = raw_csr_wtype;
	d_csr_w_imm       = raw_csr_w_imm;
	d_branchcond      = raw_branchcond;
	d_addr_is_regoffs = raw_addr_is_regoffs;
	d_except          = raw_except;
	d_sleep_wfi       = raw_sleep_wfi;
	d_sleep_block     = raw_sleep_block;
	d_sleep_unblock   = raw_sleep_unblock;
	d_fence_i         = raw_fence_i;
	d_fence_d         = raw_fence_d;

	if (d_invalid || d_starved || d_except_instr_bus_fault || partial_predicted_branch) begin
		d_rs1             = {W_REGADDR{1'b0}};
		d_rs2             = {W_REGADDR{1'b0}};
		d_rd              = {W_REGADDR{1'b0}};
		d_memop           = MEMOP_NONE;
		d_branchcond      = BCOND_NEVER;
		d_csr_ren         = 1'b0;
		d_csr_wen         = 1'b0;
		d_except          = EXCEPT_NONE;
		d_sleep_wfi       = 1'b0;
		d_sleep_block     = 1'b0;
		d_sleep_unblock   = 1'b0;
		d_fence_i         = 1'b0;
		d_fence_d         = 1'b0;

		if (EXTENSION_M)
			d_aluop = ALUOP_ADD;

		if (d_except_instr_bus_fault)
			d_except = EXCEPT_INSTR_FAULT;
		else if (d_invalid && !d_starved)
			d_except = EXCEPT_INSTR_ILLEGAL;
	end
	if (partial_predicted_branch) begin
		d_addr_is_regoffs = 1'b0;
		d_branchcond = BCOND_ALWAYS;
	end
	if (cir_lock_prev) begin
		d_branchcond = BCOND_NEVER;
	end
end

endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

`default_nettype none

module hazard3_frontend #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
) (
	input  wire              clk,
	input  wire              rst_n,

	// Fetch interface
	// addr_vld may be asserted at any time, but after assertion,
	// neither addr nor addr_vld may change until the cycle after addr_rdy.
	// There is no backpressure on the data interface; the front end
	// must ensure it does not request data it cannot receive.
	// addr_rdy and dat_vld may be functions of hready, and
	// may not be used to compute combinational outputs.
	output wire              mem_size, // 1'b1 -> 32 bit access
	output wire [W_ADDR-1:0] mem_addr,
	output wire              mem_priv,
	output wire              mem_addr_vld,
	input  wire              mem_addr_rdy,
	input  wire [W_DATA-1:0] mem_data,
	input  wire              mem_data_err,
	input  wire              mem_data_vld,

	// Jump/flush interface
	// Processor may assert vld at any time. The request will not go through
	// unless rdy is high. Processor *may* alter request during this time.
	// Inputs must not be a function of hready.
	input  wire [W_ADDR-1:0] jump_target,
	input  wire              jump_priv,
	input  wire              jump_target_vld,
	output wire              jump_target_rdy,

	// Interface to the branch target buffer. `src_addr` is the address of the
	// last halfword of a taken backward branch. The frontend redirects fetch
	// such that `src_addr` appears to be sequentially followed by `target`.
	input  wire              btb_set,
	input  wire [W_ADDR-1:0] btb_set_src_addr,
	input  wire              btb_set_src_size,
	input  wire [W_ADDR-1:0] btb_set_target_addr,
	input  wire              btb_clear,
	output wire [W_ADDR-1:0] btb_target_addr_out,

	// Interface to Decode
	output reg  [31:0]       cir,                  // Current instruction register; pre-expanded to 32-bit
	output wire [31:0]       cir_raw,              // Unexpanded instruction data
	output reg  [1:0]        cir_vld,              // number of valid halfwords in CIR
	input  wire [1:0]        cir_use,              // number of halfwords D intends to consume
	                                               // *may* be a function of hready
	output wire [1:0]        cir_err,              // Bus error on upper/lower halfword of CIR.
	output wire [1:0]        cir_predbranch,       // Set for last halfword of a predicted-taken branch
	output wire              cir_break_any,        // Set for exact match of a breakpoint address on CIR LSB
	output wire              cir_break_d_mode,     // As above but specifically break to debug mode
	output reg               cir_is_32bit,         // Can't be decoded from CIR due to pre-expansion
	output reg               cir_invalid_16bit,    // Expanded an invalid 32-bit instruction
	output reg               cir_is_uop,           // Current instruction is part of a micro-op sequence
	output reg               cir_uop_nonfinal,     // ...and there are more to follow in this instruction
	output reg               cir_uop_no_pc_update, // Suppress PC increment or jump (note the jump in cm.popret is not the final uop!)
	output reg               cir_uop_atomic,       // Prevent IRQ entry, so intermediate states are not observed
	input  wire              uop_stall,
	input  wire              uop_clear,

	// "flush_behind": do not flush the oldest instruction when accepting a
	//  jump request (but still flush younger instructions). Sometimes a
	//  stalled instruction may assert a jump request, because e.g. the stall
	//  is dependent on a bus stall signal so can't gate the request.
	input  wire              cir_flush_behind,
	// Required for regnum predecode when Zilsd is enabled:
	input  wire              df_lspair_phase_next,

	// Signal to power controller that power down is safe. (When going to
	// sleep, first the pipeline is stalled, and then the power controller
	// waits for the frontend to naturally come to a halt before releasing
	// its power request. This avoids manually halting the frontend.)
	output wire              pwrdown_ok,
	// Signal to delay the first instruction fetch following reset, because
	// powerup has not yet been negotiated.
	input  wire              delay_first_fetch,

	// Provide the rs1/rs2 register numbers which will be in CIR next cycle.
	// Coarse: valid if this instruction has a nonzero register operand.
	// (Suitable for regfile read)
	output reg  [4:0]        predecode_rs1_coarse,
	output reg  [4:0]        predecode_rs2_coarse,
	// Fine: like coarse, but accurate zeroing when the operand is implicit.
	// (Suitable for bypass. Still not precise enough for stall logic.)
	output reg  [4:0]        predecode_rs1_fine,
	output reg  [4:0]        predecode_rs2_fine,

	// Debugger instruction injection: instruction fetch is suppressed when in
	// debug halt state, and the DM can then inject instructions into the last
	// entry of the prefetch queue using the vld/rdy handshake.
	input  wire              debug_mode,
	input  wire [W_DATA-1:0] dbg_instr_data,
	input  wire              dbg_instr_data_vld,
	output wire              dbg_instr_data_rdy,

	// PMP query->kill interface for X permission checks
	output wire [W_ADDR-1:0] pmp_i_addr,
	output wire              pmp_i_m_mode,
	input  wire              pmp_i_kill,

	// Trigger unit query->break interface for breakpoints
	output wire [W_ADDR-1:0] trigger_addr,
	output wire              trigger_m_mode,
	input  wire [1:0]        trigger_break_any,
	input  wire [1:0]        trigger_break_d_mode

);

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

localparam RV_RS1_LSB = 15;
localparam RV_RS1_BITS = 5;
localparam RV_RS2_LSB = 20;
localparam RV_RS2_BITS = 5;
localparam RV_RD_LSB = 7;
localparam RV_RD_BITS = 5;

// Note: these are preprocessor macros, rather than the usual localparams,
// because it's quite difficult to get a definitive citation from 1364-2005
// for whether Z values are propagated through a localparam to a casez.
// Multiple tools complain about it, so just this once I'll use macros.





































































































































































































































































localparam W_BUNDLE = 16;
// This is the minimum for full throughput (enough to avoid dropping data when
// decode stalls) and there is no significant advantage to going larger.
localparam FIFO_DEPTH = 2;

// ----------------------------------------------------------------------------
// Fetch queue

wire jump_now = jump_target_vld && jump_target_rdy;
reg [1:0] mem_data_hwvld;

// PMP X faults are checked in parallel with the fetch (fine if executable
// memory is read-idempotent) and failures are promoted to bus errors:
wire pmp_kill_fetch_dph;
wire mem_or_pmp_err = mem_data_err || pmp_kill_fetch_dph;

// Similarly, breakpoint matches are checked during fetch data phase. These
// are called mem_xxx because they are the breakpoint metadata for the data
// coming back from memory in this dphase.
wire [1:0] mem_break_any;
wire [1:0] mem_break_d_mode;

// Mark data as containing a predicted-taken branch instruction so that
// mispredicts can be recovered -- need to track both halfwords so that we
// can mark the entire instruction, and nothing but the instruction:
reg [1:0] mem_data_predbranch;

// Bus errors (and other metadata) travel alongside data. They cause an
// exception if the core decodes the instruction, but until then can be
// flushed harmlessly.

reg  [W_DATA-1:0]    fifo_mem          [0:FIFO_DEPTH];
reg                  fifo_err          [0:FIFO_DEPTH];
reg  [1:0]           fifo_break_any    [0:FIFO_DEPTH];
reg  [1:0]           fifo_break_d_mode [0:FIFO_DEPTH];
reg  [1:0]           fifo_predbranch   [0:FIFO_DEPTH];
reg  [1:0]           fifo_valid_hw     [0:FIFO_DEPTH];
reg                  fifo_valid        [0:FIFO_DEPTH];
reg                  fifo_valid_m1     [0:FIFO_DEPTH];

wire [W_DATA-1:0] fifo_rdata       = fifo_mem[0];
wire              fifo_full        = fifo_valid[FIFO_DEPTH - 1];
wire              fifo_empty       = !fifo_valid[0];
wire              fifo_almost_full = fifo_valid[FIFO_DEPTH - 2];

wire              fifo_push;
wire              fifo_pop;
wire              fifo_dbg_inject = DEBUG_SUPPORT && dbg_instr_data_vld && dbg_instr_data_rdy;

always @ (*) begin: boundary_conditions
	integer i;
	fifo_mem[FIFO_DEPTH] = mem_data;
	fifo_predbranch[FIFO_DEPTH] = 2'b00;
	fifo_err[FIFO_DEPTH] = 1'b0;
	fifo_break_any[FIFO_DEPTH] = 2'b00;
	fifo_break_d_mode[FIFO_DEPTH] = 2'b00;
	for (i = 0; i <= FIFO_DEPTH; i = i + 1) begin
		fifo_valid[i] = |EXTENSION_C ? |fifo_valid_hw[i] : fifo_valid_hw[i][0];
		// valid-to-right condition: i == 0 || fifo_valid[i - 1], but without
		// using negative array bound (seems broken in Yosys?) or OOB in the
		// short circuit case (gives lint although result is well-defined)
		if (i == 0) begin
			fifo_valid_m1[i] = 1'b1;
		end else begin
			fifo_valid_m1[i] = fifo_valid[i - 1];
		end
	end
end

always @ (posedge clk or negedge rst_n) begin: fifo_update
	integer i;
	if (!rst_n) begin
		for (i = 0; i < FIFO_DEPTH; i = i + 1) begin
			fifo_valid_hw[i] <= 2'b00;
			fifo_mem[i] <= 32'd0;
			fifo_err[i] <= 1'b0;
			fifo_break_any[i] <= 2'b00;
			fifo_break_d_mode[i] <= 2'b00;
			fifo_predbranch[i] <= 2'b00;
		end
		// This exists only for loop boundary conditions, but is tied off in
		// this synchronous process to work around a Verilator scheduling
		// issue (see issue #21)
		fifo_valid_hw[FIFO_DEPTH] <= 2'b00;
	end else begin
		for (i = 0; i < FIFO_DEPTH; i = i + 1) begin
			if (fifo_pop || (fifo_push && !fifo_valid[i])) begin
				fifo_mem[i]          <= fifo_valid[i + 1] ? fifo_mem[i + 1]          : mem_data;
				fifo_err[i]          <= fifo_valid[i + 1] ? fifo_err[i + 1]          : mem_or_pmp_err;
				fifo_break_any[i]    <= fifo_valid[i + 1] ? fifo_break_any[i + 1]    : mem_break_any;
				fifo_break_d_mode[i] <= fifo_valid[i + 1] ? fifo_break_d_mode[i + 1] : mem_break_d_mode;
				fifo_predbranch[i]   <= fifo_valid[i + 1] ? fifo_predbranch[i + 1]   : mem_data_predbranch;
			end
			fifo_valid_hw[i] <=
				jump_now                                   ? 2'h0                            :
				fifo_valid[i + 1] && fifo_pop              ? fifo_valid_hw[i + 1]            :
				fifo_valid[i]     && fifo_pop              ? mem_data_hwvld & {2{fifo_push}} :
				fifo_valid[i]                              ? fifo_valid_hw[i]                :
				fifo_push && !fifo_pop && fifo_valid_m1[i] ? mem_data_hwvld                  : 2'h0;
		end
		// Allow DM to inject instructions directly into the lowest-numbered
		// queue entry. This mux should not extend critical path since it is
		// balanced with the instruction-assembly muxes on the queue bypass
		// path. Note that flush takes precedence over debug injection
		// (and the debug module design must account for this)
		if (fifo_dbg_inject) begin
			fifo_mem[0] <= dbg_instr_data;
			fifo_err[0] <= 1'b0;
			fifo_predbranch[0] <= 2'b00;
			fifo_break_any[0] <= 2'b00;
			fifo_break_d_mode[0] <= 2'b00;
			fifo_valid_hw[0] <= jump_now ? 2'b00 : 2'b11;
		end
		fifo_valid_hw[FIFO_DEPTH] <= 2'b00;
	end
end










assign pwrdown_ok = (fifo_full && !jump_target_vld) || debug_mode;

// ----------------------------------------------------------------------------
// Branch target buffer

wire [W_ADDR-1:0] btb_src_addr;
wire              btb_src_size;
wire [W_ADDR-1:0] btb_target_addr;
wire              btb_valid;

generate
if (BRANCH_PREDICTOR) begin: have_btb
	reg [W_ADDR-1:0] btb_src_addr_r;
	reg              btb_src_size_r;
	reg [W_ADDR-1:0] btb_target_addr_r;
	reg              btb_valid_r;
	assign btb_src_addr    = btb_src_addr_r;
	assign btb_src_size    = btb_src_size_r;
	assign btb_target_addr = btb_target_addr_r;
	assign btb_valid       = btb_valid_r;
	always @ (posedge clk or negedge rst_n) begin
		if (!rst_n) begin
			btb_src_addr_r <= {W_ADDR{1'b0}};
			btb_src_size_r <= 1'b0;
			btb_target_addr_r <= {W_ADDR{1'b0}};
			btb_valid_r <= 1'b0;
		end else if (btb_clear) begin
			// Clear takes precedences over set. E.g. if a taken branch is in
			// stage 2 and an exception is in stage 3, we must clear the BTB.
			btb_valid_r <= 1'b0;
		end else if (btb_set) begin
			btb_src_addr_r <= btb_set_src_addr;
			btb_src_size_r <= btb_set_src_size;
			btb_target_addr_r <= btb_set_target_addr;
			btb_valid_r <= 1'b1;
		end
	end
end else begin: no_btb
	assign btb_src_addr = {W_ADDR{1'b0}};
	assign btb_src_size = 1'b0;
	assign btb_target_addr = {W_ADDR{1'b0}};
	assign btb_valid = 1'b0;
end
endgenerate

// Decode uses the target address to set the PC to the correct branch target
// value following a predicted-taken branch (as normally it would update PC
// by following an X jump request, and in this case there is none).
//
// Note this assumes the BTB target has not changed by the time the predicted
// branch arrives at decode! This is always true because the only way for the
// target address to change is when an older branch is taken, which would
// flush the younger predicted-taken branch before it reaches decode.

assign btb_target_addr_out = btb_target_addr;

// ----------------------------------------------------------------------------
// Fetch request generation

// Fetch addr runs ahead of the PC, in word increments.
reg [W_ADDR-1:0] fetch_addr;
reg              fetch_priv;
reg              btb_prev_start_of_overhanging;
reg [1:0]        mem_aph_hwvld;
reg              mem_addr_hold;

wire btb_match_word = |BRANCH_PREDICTOR && btb_valid && (
	fetch_addr[W_ADDR-1:2] == btb_src_addr[W_ADDR-1:2]
);

// Catch case where predicted-taken branch instruction extends into next word:
wire btb_src_overhanging    = btb_src_size && btb_src_addr[1];

// Suppress case where we have jumped immediately after a word-aligned halfword-sized
// branch, and the jump target went into fetch_addr due to an address-phase hold:
wire btb_jumped_beyond      = !btb_src_size && !btb_src_addr[1] && !mem_aph_hwvld[0];

wire btb_match_current_addr = btb_match_word && !btb_src_overhanging && !btb_jumped_beyond;
wire btb_match_next_addr    = btb_match_word && btb_src_overhanging;

wire btb_match_now = btb_match_current_addr || btb_prev_start_of_overhanging;

// Post-increment if jump request is going straight through
wire [W_ADDR-1:0] jump_target_post_increment =
	{jump_target[W_ADDR-1:2],                          2'b00} +
	{{W_ADDR-3{1'b0}}, mem_addr_rdy && !mem_addr_hold, 2'b00};

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		fetch_addr <= RESET_VECTOR;
		// M-mode at reset:
		fetch_priv <= 1'b1;
		btb_prev_start_of_overhanging <= 1'b0;
	end else begin
		if (jump_now) begin
			fetch_addr <= jump_target_post_increment;
			fetch_priv <= jump_priv || !U_MODE;
			btb_prev_start_of_overhanging <= 1'b0;
		end else if (mem_addr_vld && mem_addr_rdy) begin
			if (btb_match_now && |BRANCH_PREDICTOR) begin
				fetch_addr <= {btb_target_addr[W_ADDR-1:2], 2'b00};
			end else begin
				fetch_addr <= fetch_addr + 32'd4;
			end
			btb_prev_start_of_overhanging <= btb_match_next_addr;
		end
	end
end

// Combinatorially generate the address-phase request

reg reset_holdoff;
always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		reset_holdoff <= 1'b1;
	end else begin
		reset_holdoff <= (|EXTENSION_XH3POWER && delay_first_fetch) ? reset_holdoff : 1'b0;
		// This should be impossible, but assert to be sure, because it *will*
		// change the fetch address (and we shouldn't check it in hardware if
		// we can prove it doesn't happen)
	end
end







reg [W_ADDR-1:0] mem_addr_r;
reg              mem_priv_r;
reg              mem_addr_vld_r;

// Downstream accesses are always word-sized word-aligned.
assign mem_addr = mem_addr_r;
assign mem_priv = mem_priv_r;
assign mem_addr_vld = mem_addr_vld_r && !reset_holdoff;
assign mem_size = 1'b1;

wire fetch_stall;

always @ (*) begin
	mem_addr_r = fetch_addr;
	mem_priv_r = fetch_priv;
	mem_addr_vld_r = 1'b1;
	case (1'b1)
		mem_addr_hold                    : begin mem_addr_r = fetch_addr; end
		jump_target_vld || reset_holdoff : begin
		                                         mem_addr_r = {jump_target[W_ADDR-1:2], 2'b00};
		                                         mem_priv_r = jump_priv || !U_MODE;
		end
		DEBUG_SUPPORT && debug_mode      : begin mem_addr_vld_r = 1'b0; end
		!fetch_stall                     : begin mem_addr_r = fetch_addr; end
		default                          : begin mem_addr_vld_r = 1'b0; end
	endcase
end

assign jump_target_rdy = !mem_addr_hold;

// ----------------------------------------------------------------------------
// Bus Pipeline Tracking

// Keep track of some useful state of the memory interface

reg  [1:0] pending_fetches;
reg  [1:0] ctr_flush_pending;

wire [1:0] pending_fetches_next = pending_fetches + (mem_addr_vld && !mem_addr_hold) - mem_data_vld;

// Using the non-registered version of pending_fetches would improve FIFO
// utilisation, but create a combinatorial path from hready to address phase!
// This means at least a 2-word FIFO is required for full fetch throughput.
assign fetch_stall = fifo_full
	|| fifo_almost_full && |pending_fetches
	|| pending_fetches > 2'h1;

// Debugger only injects instructions when the frontend is at rest and empty.
assign dbg_instr_data_rdy = DEBUG_SUPPORT && !fifo_valid[0] && ~|ctr_flush_pending;

wire cir_room_for_fetch;
// If fetch data is forwarded past the FIFO, ensure it is not also written to it.
assign fifo_push = mem_data_vld && ~|ctr_flush_pending && !(cir_room_for_fetch && fifo_empty)
	&& !(DEBUG_SUPPORT && debug_mode);

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mem_addr_hold <= 1'b0;
		pending_fetches <= 2'h0;
		ctr_flush_pending <= 2'h0;
	end else begin
		mem_addr_hold <= mem_addr_vld && !mem_addr_rdy;
		pending_fetches <= pending_fetches_next;
		if (jump_now) begin
			ctr_flush_pending <= pending_fetches - mem_data_vld;
		end else if (|ctr_flush_pending && mem_data_vld) begin
			ctr_flush_pending <= ctr_flush_pending - 1'b1;
		end
	end
end









always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		mem_data_hwvld <= 2'b11;
		mem_aph_hwvld <= 2'b11;
		mem_data_predbranch <= 2'b00;
	end else begin
		if (jump_now) begin
			if (|EXTENSION_C) begin
				if (mem_addr_rdy) begin
					mem_aph_hwvld <= 2'b11;
					mem_data_hwvld <= {1'b1, !jump_target[1]};
				end else begin
					mem_aph_hwvld <= {1'b1, !jump_target[1]};
				end
			end
			mem_data_predbranch <= 2'b00;
		end else if (mem_addr_vld && mem_addr_rdy) begin
			if (|EXTENSION_C) begin
				// If a predicted-taken branch instruction only spans the first
				// half of a word, need to flag the second half as invalid.
				mem_data_hwvld <= mem_aph_hwvld & {
					!(|BRANCH_PREDICTOR && btb_match_now && (btb_src_addr[1] == btb_src_size)),
					1'b1
				};
				// Also need to take the alignment of the destination into account.
				mem_aph_hwvld <= {
					1'b1,
					!(|BRANCH_PREDICTOR && btb_match_now && btb_target_addr[1])
				};
			end
			mem_data_predbranch <=
				|BRANCH_PREDICTOR && btb_match_word ? (
					btb_src_addr[1] ? 2'b10 :
					btb_src_size    ? 2'b11 : 2'b01
				) :
				|BRANCH_PREDICTOR && btb_prev_start_of_overhanging ? (
					2'b01
				) : 2'b00;
		end
	end
end

// ----------------------------------------------------------------------------
// PMP and trigger unit interfacing: query -> kill/break

wire [W_ADDR-1:0] pmp_trigger_check_dph_addr;
wire              pmp_trigger_check_dph_m_mode;

// Register the fetch address into stage F so that the PMP can check it in
// parallel with the bus data phase. Feels wasteful to have a separate
// register, but using the fetch_addr counter is fraught due to the way that
// new addresses go into it or past it (depending on aphase hold).

generate
if (PMP_REGIONS > 0 || DEBUG_SUPPORT != 0) begin: have_check_reg

	reg [W_ADDR-1:0] check_addr_dph;
	reg              check_m_mode_dph;
	always @ (posedge clk or negedge rst_n) begin
		if (!rst_n) begin
			check_addr_dph <= {W_ADDR{1'b0}};
			check_m_mode_dph <= 1'b0;
		end else if (mem_addr_vld && mem_addr_rdy) begin
			check_addr_dph <= mem_addr;
			check_m_mode_dph <= mem_priv;
		end
	end

	assign pmp_trigger_check_dph_addr = check_addr_dph;
	assign pmp_trigger_check_dph_m_mode = check_m_mode_dph;

end else begin: no_check_reg

	assign pmp_trigger_check_dph_addr = {W_ADDR{1'b0}};
	assign pmp_trigger_check_dph_m_mode = 1'b0;

end
endgenerate

generate
if (PMP_REGIONS == 0) begin: no_pmp

	assign pmp_i_addr = {W_ADDR{1'b0}};
	assign pmp_i_m_mode = 1'b0;
	assign pmp_kill_fetch_dph = 1'b0;

end else begin: have_pmp

	assign pmp_i_addr = pmp_trigger_check_dph_addr;
	assign pmp_i_m_mode = pmp_trigger_check_dph_m_mode;
	assign pmp_kill_fetch_dph = pmp_i_kill && !debug_mode;

end
endgenerate

generate
if (DEBUG_SUPPORT == 0) begin: no_triggers

	assign trigger_addr = {W_ADDR{1'b0}};
	assign trigger_m_mode = 1'b0;
	assign mem_break_any = 2'b00;
	assign mem_break_d_mode = 2'b00;

end else begin: have_triggers

	assign trigger_addr = pmp_trigger_check_dph_addr;
	assign trigger_m_mode = pmp_trigger_check_dph_m_mode;
	assign mem_break_any = trigger_break_any & {|EXTENSION_C, 1'b1};
	assign mem_break_d_mode = trigger_break_d_mode & {|EXTENSION_C, 1'b1};

end
endgenerate

// ----------------------------------------------------------------------------
// Instruction buffer

// The instruction buffer is a 3 x ~16-bit shift register:
//
// * 2 x 16-bit entries form the 32-bit current instruction register (CIR)
//   which is the processor's decode window
//
// * 1 x 16-bit entry allows the decode window to be non-32-bit-aligned with
//   respect to the 2 x 32-bit prefetch queue entries, which are always
//   naturally aligned in memory (if fully populated).
//
// The third entry should be trimmed for non-RVC configurations due to
// constant-folding on EXTENSION_C; it is unnecessary here because the
// instructions are always 32-bit-aligned.

// The entries ("slots") are slightly larger than 16 bits because they also
// contain metadata like bus errors:
localparam W_SLOT                = 4 + W_BUNDLE;
localparam SLOT_BREAK_ANY_BIT    = 3 + W_BUNDLE;
localparam SLOT_BREAK_D_MODE_BIT = 2 + W_BUNDLE;
localparam SLOT_ERR_BIT          = 1 + W_BUNDLE;
localparam SLOT_PREDBRANCH_BIT   = 0 + W_BUNDLE;

reg  [3*W_SLOT-1:0] buf_contents;
reg  [1:0]          buf_level;

wire                fetch_data_vld   = !fifo_empty || (mem_data_vld && ~|ctr_flush_pending && !debug_mode);

wire [W_DATA-1:0]   fetch_data         = fifo_empty ? mem_data            : fifo_rdata;
wire [1:0]          fetch_data_hwvld   = fifo_empty ? mem_data_hwvld      : fifo_valid_hw[0];
wire                fetch_bus_err      = fifo_empty ? mem_or_pmp_err      : fifo_err[0];
wire [1:0]          fetch_break_any    = fifo_empty ? mem_break_any       : fifo_break_any[0];
wire [1:0]          fetch_break_d_mode = fifo_empty ? mem_break_d_mode    : fifo_break_d_mode[0];
wire [1:0]          fetch_predbranch   = fifo_empty ? mem_data_predbranch : fifo_predbranch[0];

wire [W_SLOT-1:0] fetch_contents_hw1 = {
	fetch_break_any[1],
	fetch_break_d_mode[1],
	fetch_bus_err,
	fetch_predbranch[1],
	fetch_data[W_BUNDLE +: W_BUNDLE]
};

wire [W_SLOT-1:0] fetch_contents_hw0 = {
	fetch_break_any[0],
	fetch_break_d_mode[0],
	fetch_bus_err,
	fetch_predbranch[0],
	fetch_data[0 +: W_BUNDLE]
};

wire [2*W_SLOT-1:0] fetch_contents_aligned = {
	fetch_contents_hw1,
	fetch_data_hwvld[0] || ~|EXTENSION_C ? fetch_contents_hw0 : fetch_contents_hw1
};

// Shift not-yet-used contents down to backfill D's consumption. We don't care
// about anything which is invalid or will be overlaid with fresh data, so
// choose these values in a way that minimises muxes.
wire [3*W_SLOT-1:0] buf_shifted =
	cir_use[1]                ? {buf_contents[W_SLOT +: 2 * W_SLOT], buf_contents[2 * W_SLOT +: W_SLOT]} :
	cir_use[0] && EXTENSION_C ? {buf_contents[2 * W_SLOT +: W_SLOT], buf_contents[W_SLOT +: 2 * W_SLOT]} :
	                            buf_contents;

wire [1:0] level_next_no_fetch = buf_level - cir_use;

// Overlay fresh fetch data onto the shifted/recycled buffer contents. Again,
// if something won't be looked at, generate the cheapest possible garbage.
assign cir_room_for_fetch = level_next_no_fetch <= (|EXTENSION_C && ~&fetch_data_hwvld ? 2'h2 : 2'h1);
assign fifo_pop = cir_room_for_fetch && !fifo_empty;

wire [3*W_SLOT-1:0] buf_shifted_plus_fetch =
	!cir_room_for_fetch                    ? buf_shifted :
	level_next_no_fetch[1] && |EXTENSION_C ? {fetch_contents_aligned[0 +: W_SLOT], buf_shifted[0 +: 2 * W_SLOT]} :
	level_next_no_fetch[0] && |EXTENSION_C ? {fetch_contents_aligned, buf_shifted[0 +: W_SLOT]} :
	                                         {buf_shifted[2 * W_SLOT +: W_SLOT], fetch_contents_aligned};

wire [1:0] fetch_fill_amount = cir_room_for_fetch && fetch_data_vld ? (
	&fetch_data_hwvld || ~|EXTENSION_C ? 2'h2 : 2'h1
) : 2'h0;

wire [1:0] buf_level_next = {1'b1, |EXTENSION_C} & (
	jump_now && cir_flush_behind ? (cir_is_32bit ? 2'h2 : 2'h1) :
	jump_now                     ? 2'h0 : level_next_no_fetch + fetch_fill_amount
);

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		buf_level <= 2'h0;
		cir_vld <= 2'h0;
		// Mysterious reset value ensures address buses are zero in reset
		// (see definition of d_addr_offs in hazard3_decode)
		buf_contents <= {{3 * W_SLOT - 2{1'b0}}, 2'b11};
	end else begin
		buf_level <= buf_level_next;
		cir_vld <= buf_level_next & ~(buf_level_next >> 1'b1);
		buf_contents <= buf_shifted_plus_fetch;
	end
end



















assign cir_err = {
	buf_contents[1 * W_SLOT + SLOT_ERR_BIT],
	buf_contents[0 * W_SLOT + SLOT_ERR_BIT]
};

assign cir_predbranch = {
	buf_contents[1 * W_SLOT + SLOT_PREDBRANCH_BIT],
	buf_contents[0 * W_SLOT + SLOT_PREDBRANCH_BIT]
};

assign cir_break_any = buf_contents[0 * W_SLOT + SLOT_BREAK_ANY_BIT] && |cir_vld;

assign cir_break_d_mode = buf_contents[0 * W_SLOT + SLOT_BREAK_D_MODE_BIT] && |cir_vld;

// ----------------------------------------------------------------------------
// Register number predecode

wire [31:0] next_instr = {
	buf_shifted_plus_fetch[1 * W_SLOT +: W_BUNDLE],
	buf_shifted_plus_fetch[0 * W_SLOT +: W_BUNDLE]
};

wire next_instr_is_32bit = next_instr[1:0] == 2'b11 || ~|EXTENSION_C;

wire [3:0] decomp_uop_step;
wire [3:0] uop_ctr = decomp_uop_step & {4{|EXTENSION_ZCMP}};

wire [4:0] zcmp_pushpop_rs2 =
	uop_ctr == 4'h0 ? 5'd01                   : // ra
	uop_ctr == 4'h1 ? 5'd08                   : // s0
	uop_ctr == 4'h2 ? 5'd09                   : // s1
	                  5'd15 + {1'b0, uop_ctr} ; // s2-s11

wire [4:0] zcmp_pushpop_rs1 =
	uop_ctr <  4'hd ? 5'd02 :                   // sp   (addr base reg)
	uop_ctr == 4'hd ? 5'd00 :                   // zero (clear a0)
	uop_ctr == 4'he ? 5'd01 :                   // ra   (ret)
	                  5'd02 ;                   // sp   (stack adj)

wire [4:0] zcmp_sa01_r1s  = {|next_instr[9:8], ~|next_instr[9:8], next_instr[9:7]};
wire [4:0] zcmp_sa01_r2s  = {|next_instr[4:3], ~|next_instr[4:3], next_instr[4:2]};

wire [4:0] zcmp_mvsa01_rs1 = {4'h5, uop_ctr[0]};
wire [4:0] zcmp_mva01s_rs1 = uop_ctr[0] ? zcmp_sa01_r2s : zcmp_sa01_r1s;

// "coarse" because the mapping of pair (x0, x1) -> (x0, x0) is not yet applied
wire [4:0] zilsd_rs2_coarse      = {       next_instr[24:21], df_lspair_phase_next ^ ~next_instr[15]};
wire [4:0] zclsd_sd_rs2_coarse   = {2'b01, next_instr[4:3],   df_lspair_phase_next ^ ~next_instr[7] };
wire [4:0] zclsd_sdsp_rs2_coarse = {       next_instr[6:3],   df_lspair_phase_next ^ 1'b1           };

always @ (*) begin

	casez ({next_instr_is_32bit, |EXTENSION_ZCMP, next_instr[15:0]})
	{1'b1, 1'bz, 16'bzzzzzzzzzzzzzzzz}: predecode_rs1_coarse = next_instr[19:15]; // 32-bit R, S, B formats
	{1'b0, 1'bz, 16'b00zzzzzzzzzzzz00}: predecode_rs1_coarse = 5'd2;              // c.addi4spn + don't care
	{1'b0, 1'bz, 16'b0zzzzzzzzzzzzz01}: predecode_rs1_coarse = next_instr[11:7];  // c.addi, c.addi16sp + don't care (jal, li)
	{1'b0, 1'bz, 16'bz1zzzzzzzzzzzz10}: predecode_rs1_coarse = 5'd2;              // c.lwsp, c.swsp, c.ldsp, c.sdsp
	{1'b0, 1'bz, 16'bz00zzzzzzzzzzz10}: predecode_rs1_coarse = next_instr[11:7];  // c.slli, c.mv, c.add
	{1'b0, 1'b1, 16'b1011zzzzzzzzzz10}: predecode_rs1_coarse = zcmp_pushpop_rs1;  // cm.push, cm.pop*
	{1'b0, 1'b1, 16'b1010zzzzz0zzzz10}: predecode_rs1_coarse = zcmp_mvsa01_rs1;   // cm.mvsa01
	{1'b0, 1'b1, 16'b1010zzzzz1zzzz10}: predecode_rs1_coarse = zcmp_mva01s_rs1;   // cm.mva01s
	default:                            predecode_rs1_coarse = {2'b01, next_instr[9:7]};
	endcase

	casez ({next_instr_is_32bit, |EXTENSION_ZCMP, |EXTENSION_ZILSD, |EXTENSION_ZCLSD, next_instr[15:0]})
	{1'b1, 1'bz, 1'b1, 1'bz, 16'bzz11zzzzz0z0zzzz}: predecode_rs2_coarse = zilsd_rs2_coarse;   // ld, sd (Zilsd)
	{1'b1, 1'bz, 1'b0, 1'bz, 16'bzz11zzzzz0z0zzzz}: predecode_rs2_coarse = next_instr[24:20];  // ld, sd (no Zilsd)

	{1'b1, 1'bz, 1'bz, 1'bz, 16'bzz0zzzzzzzzzzzzz}: predecode_rs2_coarse = next_instr[24:20];  // (cover remaining 32-bit
	{1'b1, 1'bz, 1'bz, 1'bz, 16'bzz10zzzzzzzzzzzz}: predecode_rs2_coarse = next_instr[24:20];  //  patterns, without overlap)
	{1'b1, 1'bz, 1'bz, 1'bz, 16'bzz11zzzzz1zzzzzz}: predecode_rs2_coarse = next_instr[24:20];
	{1'b1, 1'bz, 1'bz, 1'bz, 16'bzz11zzzzz0z1zzzz}: predecode_rs2_coarse = next_instr[24:20];

	{1'b0, 1'bz, 1'b1, 1'b1, 16'bzz1zzzzzzzzzzz00}: predecode_rs2_coarse = zclsd_sd_rs2_coarse;
	{1'b0, 1'bz, 1'bz, 1'bz, 16'bzz0zzzzzzzzzzz10}: predecode_rs2_coarse = next_instr[6:2];    // c.add, c.swsp
	{1'b0, 1'b1, 1'bz, 1'bz, 16'bz01zzzzzzzzzzz10}: predecode_rs2_coarse = zcmp_pushpop_rs2;   // cm.push
	{1'b0, 1'bz, 1'b1, 1'b1, 16'bz11zzzzzzzzzzz10}: predecode_rs2_coarse = zclsd_sdsp_rs2_coarse;
	default:                                        predecode_rs2_coarse = {2'b01, next_instr[4:2]};
	endcase

	// The "fine" predecode targets those instructions which either:
	// - Have an implicit zero-register operand in their expanded form (e.g. c.beqz)
	// - Do not have a register operand on that port, but rely on the port being 0
	// We don't care about instructions which ignore the reg ports, e.g. ebreak

	casez ({|EXTENSION_C, next_instr})
	// -> addi rd, x0, imm:
	{1'b1, 16'hzzzz, 16'b010???????????01}: predecode_rs1_fine = 5'd0;
	{1'b1, 16'hzzzz, 16'b1000??????????10}: begin
		if (next_instr[6:2] == 5'd0) begin
			// c.jr has rs1 as normal
			predecode_rs1_fine = predecode_rs1_coarse;
		end else begin
			// -> add rd, x0, rs2:
			predecode_rs1_fine = 5'd0;
		end
	end
	default: predecode_rs1_fine = predecode_rs1_coarse;
	endcase

	casez ({|EXTENSION_C, |EXTENSION_ZILSD, |EXTENSION_ZCLSD, next_instr})
	{1'b1, 1'bz, 1'bz, 16'hzzzz, 16'b110???????????01}: predecode_rs2_fine = 5'd0;    // -> beq rs1, x0, label
	{1'b1, 1'bz, 1'bz, 16'hzzzz, 16'b111???????????01}: predecode_rs2_fine = 5'd0;    // -> bne rs1, x0, label
	{1'b1, 1'b1, 1'bz,           32'b???????????0?????011?????0100011    }: predecode_rs2_fine = predecode_rs2_coarse & {5{|next_instr[24:21]}};
	{1'b1, 1'b1, 1'b1, 16'hzzzz, 16'b111??????????010}: predecode_rs2_fine = predecode_rs2_coarse & {5{|next_instr[ 6: 3]}};
	default:                                     predecode_rs2_fine = predecode_rs2_coarse;
	endcase

	if (|EXTENSION_E) begin
		predecode_rs1_coarse[4] = 1'b0;
		predecode_rs2_coarse[4] = 1'b0;
		predecode_rs1_fine[4] = 1'b0;
		predecode_rs2_fine[4] = 1'b0;
	end

end

// ----------------------------------------------------------------------------
// Instruction decompression

// Instructions are decompressed at the end of stage 1 (fetch data phase). On
// ASIC, where the register file is synthesised with muxes, this puts
// decompression somewhat in parallel with register file read, which uses
// approximately decoded regnums.

generate
if (~|EXTENSION_C) begin: no_decompress

	// No decompression; instructions decoded directly from prefetch buffer
	always @ (*) begin
		cir = {
			buf_contents[1 * W_SLOT +: W_BUNDLE],
			buf_contents[0 * W_SLOT +: W_BUNDLE]
		} | 32'd3;
		cir_is_32bit         = 1'b1;
		cir_invalid_16bit    = ~&buf_contents[1:0];
		cir_is_uop           = 1'b0;
		cir_uop_nonfinal     = 1'b0;
		cir_uop_no_pc_update = 1'b0;
		cir_uop_atomic       = 1'b0;
	end	

	assign decomp_uop_step = 4'h0;

end else begin: have_decompress

	wire        decomp_instr_is_32bit;
	wire [31:0] decomp_instr_out;
	wire        decomp_is_uop;
	wire        decomp_is_final_uop;
	wire        decomp_uop_no_pc_update;
	wire        decomp_uop_atomic;
	wire        decomp_invalid;

	wire        first_uop           = ~|decomp_uop_step;
	// Ensure the first uop goes straight through, as it is registered into CIR:
	wire        uop_stall_non_first = first_uop ? ~|buf_level_next : uop_stall;
	// Ensure the uop counter stops at 0 after rolling over once:
	wire        uop_stall_on_repeat = cir_is_uop && !cir_uop_nonfinal && ~|cir_use;

	hazard3_instr_decompress #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Pass-through of parameters defined in hazard3_config.vh, so that these can
// be set at instantiation rather than editing the config file, and will flow
// correctly down through the hierarchy.

.RESET_VECTOR        (RESET_VECTOR),
.MTVEC_INIT          (MTVEC_INIT),
.EXTENSION_A         (EXTENSION_A),
.EXTENSION_C         (EXTENSION_C),
.EXTENSION_E         (EXTENSION_E),
.EXTENSION_M         (EXTENSION_M),
.EXTENSION_ZBA       (EXTENSION_ZBA),
.EXTENSION_ZBB       (EXTENSION_ZBB),
.EXTENSION_ZBC       (EXTENSION_ZBC),
.EXTENSION_ZBKB      (EXTENSION_ZBKB),
.EXTENSION_ZBKX      (EXTENSION_ZBKX),
.EXTENSION_ZBS       (EXTENSION_ZBS),
.EXTENSION_ZCB       (EXTENSION_ZCB),
.EXTENSION_ZCLSD     (EXTENSION_ZCLSD),
.EXTENSION_ZCMP      (EXTENSION_ZCMP),
.EXTENSION_ZIFENCEI  (EXTENSION_ZIFENCEI),
.EXTENSION_ZILSD     (EXTENSION_ZILSD),
.EXTENSION_XH3BEXTM  (EXTENSION_XH3BEXTM),
.EXTENSION_XH3IRQ    (EXTENSION_XH3IRQ),
.EXTENSION_XH3PMPM   (EXTENSION_XH3PMPM),
.EXTENSION_XH3POWER  (EXTENSION_XH3POWER),
.CSR_M_MANDATORY     (CSR_M_MANDATORY),
.CSR_M_TRAP          (CSR_M_TRAP),
.CSR_COUNTER         (CSR_COUNTER),
.U_MODE              (U_MODE),
.PMP_REGIONS         (PMP_REGIONS),
.PMP_GRAIN           (PMP_GRAIN),
.PMP_MATCH_NAPOT     (PMP_MATCH_NAPOT),
.PMP_MATCH_TOR       (PMP_MATCH_TOR),
.PMP_HARDWIRED       (PMP_HARDWIRED),
.PMP_HARDWIRED_ADDR  (PMP_HARDWIRED_ADDR),
.PMP_HARDWIRED_CFG   (PMP_HARDWIRED_CFG),
.DEBUG_SUPPORT       (DEBUG_SUPPORT),
.BREAKPOINT_TRIGGERS (BREAKPOINT_TRIGGERS),
.NUM_IRQS            (NUM_IRQS),
.IRQ_PRIORITY_BITS   (IRQ_PRIORITY_BITS),
.IRQ_INPUT_BYPASS    (IRQ_INPUT_BYPASS),
.MVENDORID_VAL       (MVENDORID_VAL),
.MCONFIGPTR_VAL      (MCONFIGPTR_VAL),
.REDUCED_BYPASS      (REDUCED_BYPASS),
.MULDIV_UNROLL       (MULDIV_UNROLL),
.MUL_FAST            (MUL_FAST),
.MUL_FASTER          (MUL_FASTER),
.MULH_FAST           (MULH_FAST),
.FAST_BRANCHCMP      (FAST_BRANCHCMP),
.BRANCH_PREDICTOR    (BRANCH_PREDICTOR),
.MTVEC_WMASK         (MTVEC_WMASK),
.RESET_REGFILE       (RESET_REGFILE),
.W_ADDR              (W_ADDR),
.W_DATA              (W_DATA)
	) decomp (
		.clk                        (clk),
		.rst_n                      (rst_n),

		.instr_in                   (next_instr),

		.instr_is_32bit             (decomp_instr_is_32bit),
		.instr_out                  (decomp_instr_out),

		.instr_out_is_uop           (decomp_is_uop),
		.instr_out_is_final_uop     (decomp_is_final_uop),
		.instr_out_uop_no_pc_update (decomp_uop_no_pc_update),
		.instr_out_uop_atomic       (decomp_uop_atomic),
		.instr_out_uop_stall        (uop_stall_non_first || uop_stall_on_repeat),
		.instr_out_uop_clear        (uop_clear),
		.df_uop_step                (decomp_uop_step),

		.invalid                    (decomp_invalid)
	);

	wire cir_clken =
		~|cir_vld || (!cir_vld[1] && &buf_contents[1:0]) ||
		|cir_use  || (|EXTENSION_ZCMP && cir_is_uop && !uop_stall) ||
		(|EXTENSION_ZCMP && uop_clear);

	wire cir_is_uop_next = decomp_is_uop && |buf_level_next;
	wire cir_uop_nonfinal_next = cir_is_uop_next && !decomp_is_final_uop;

	always @ (posedge clk or negedge rst_n) begin
		if (!rst_n) begin
			cir                  <= 32'd3;
			cir_is_32bit         <= 1'b0;
			cir_invalid_16bit    <= 1'b0;
			cir_is_uop           <= 1'b0;
			cir_uop_nonfinal     <= 1'b0;
			cir_uop_no_pc_update <= 1'b0;
			cir_uop_atomic       <= 1'b0;
		end else if (cir_clken) begin
			cir                  <= decomp_instr_out | 32'd3;
			cir_is_32bit         <= decomp_instr_is_32bit;
			cir_invalid_16bit    <= decomp_invalid;
			cir_is_uop           <= |EXTENSION_ZCMP && !uop_clear && cir_is_uop_next;
			cir_uop_nonfinal     <= |EXTENSION_ZCMP && !uop_clear && cir_uop_nonfinal_next;
			cir_uop_no_pc_update <= |EXTENSION_ZCMP && !uop_clear && decomp_uop_no_pc_update;
			cir_uop_atomic       <= |EXTENSION_ZCMP && !uop_clear && decomp_uop_atomic;
		end
	end

end
endgenerate

assign cir_raw = {
	buf_contents[1 * W_SLOT +: W_BUNDLE],
	buf_contents[0 * W_SLOT +: W_BUNDLE]
};

endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2023 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Little instructions go in, big instructions come out

`default_nettype none

module hazard3_instr_decompress #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
) (
	input  wire        clk,
	input  wire        rst_n,

	input  wire [31:0] instr_in,

	output reg         instr_is_32bit,
	output reg  [31:0] instr_out,

	// If instruction is a non-final uop, need to suppress PC update, and null
	// the PC offset in the mepc address in stage 3.
	output wire        instr_out_is_uop,
	output wire        instr_out_is_final_uop,
	output wire        instr_out_uop_no_pc_update,
	// Indicate instr_out is a uop from the noninterruptible part of a uop
	// sequence. If one uop is noninterruptible, all following uops until the
	// end of the sequence are also noninterruptible.
	output wire        instr_out_uop_atomic,
	// Current ucode sequence is stalled on downstream execution
	input  wire        instr_out_uop_stall,
	input  wire        instr_out_uop_clear,

	// To regnum decoder in frontend
	output wire [3:0]  df_uop_step,

	output reg         invalid
);

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

localparam RV_RS1_LSB = 15;
localparam RV_RS1_BITS = 5;
localparam RV_RS2_LSB = 20;
localparam RV_RS2_BITS = 5;
localparam RV_RD_LSB = 7;
localparam RV_RD_BITS = 5;

// Note: these are preprocessor macros, rather than the usual localparams,
// because it's quite difficult to get a definitive citation from 1364-2005
// for whether Z values are propagated through a localparam to a casez.
// Multiple tools complain about it, so just this once I'll use macros.





































































































































































































































































localparam W_REGADDR = 5;
localparam PASSTHROUGH = ~|EXTENSION_C;

// Long-register formats: cr, ci, css
// Short-register formats: ciw, cl, cs, cb, cj
wire [W_REGADDR-1:0] rd_l  = instr_in[11:7];
wire [W_REGADDR-1:0] rs1_l = instr_in[11:7];
wire [W_REGADDR-1:0] rs2_l = instr_in[6:2];
wire [W_REGADDR-1:0] rd_s  = {2'b01, instr_in[4:2]};
wire [W_REGADDR-1:0] rs1_s = {2'b01, instr_in[9:7]};
wire [W_REGADDR-1:0] rs2_s = {2'b01, instr_in[4:2]};

// Mapping of cx -> x immediate formats (we are *expanding* instructions, not
// decoding them):

wire [31:0] imm_ci = {
	{7{instr_in[12]}},
	instr_in[6:2],
	20'h00000
};

wire [31:0] imm_cj = {
	instr_in[12],
	instr_in[8],
	instr_in[10:9],
	instr_in[6],
	instr_in[7],
	instr_in[2],
	instr_in[11],
	instr_in[5:3],
	{9{instr_in[12]}},
	12'h000
};

wire [31:0] imm_cb ={
	{4{instr_in[12]}},
	instr_in[6:5],
	instr_in[2],
	13'h0000,
	instr_in[11:10],
	instr_in[4:3],
	instr_in[12],
	7'h00
};

wire [31:0] imm_c_lb = {
	10'h0,
	instr_in[5],
	instr_in[6],
	20'h00000
};

wire [31:0] imm_c_lh = {
	10'h000,
	instr_in[5],
	1'b0,
	20'h00000
};

function [31:0] rfmt_rd;  input [4:0] rd;  begin rfmt_rd  = {20'h00000, rd, 7'h00};   end endfunction
function [31:0] rfmt_rs1; input [4:0] rs1; begin rfmt_rs1 = {12'h000, rs1, 15'h0000}; end endfunction
function [31:0] rfmt_rs2; input [4:0] rs2; begin rfmt_rs2 = {7'h00, rs2, 20'h00000};  end endfunction

// ----------------------------------------------------------------------------
// Push/pop and friends

// The longest uop sequence is a maximal cm.popretz:
//
// - 13x lw                     (counter = 0..12)
// - 1x addi to set a0 to zero  (counter = 13   ) < atomic section
// - 1x jalr to jump through ra (counter = 14   ) < atomic section
// - 1x addi to adjust sp       (counter = 15   ) < atomic section

wire [3:0] uop_ctr;
reg  [3:0] uop_ctr_nxt_in_seq;
reg        in_uop_seq;
reg        uop_no_pc_update;

wire zcmp_is_pushpop = instr_in[12];
wire uop_seq_end      = |EXTENSION_ZCMP && (zcmp_is_pushpop ? uop_ctr == 4'hf : uop_ctr[0]);
wire uop_atomic       = |EXTENSION_ZCMP && (zcmp_is_pushpop ? uop_ctr >= 4'he : uop_ctr[0]);

wire [3:0] uop_ctr_nxt =
	instr_out_uop_clear ? 4'h0    :
	instr_out_uop_stall ? uop_ctr : uop_ctr_nxt_in_seq;

assign instr_out_is_uop = in_uop_seq;
assign instr_out_is_final_uop = uop_seq_end;
assign instr_out_uop_atomic = uop_atomic;
assign instr_out_uop_no_pc_update = uop_no_pc_update;
assign df_uop_step = uop_ctr;

// The offset from current sp value to the lowest-addressed saved register, +64.
wire [3:0] zcmp_rlist = instr_in[7:4];
wire [3:0] zcmp_n_regs = zcmp_rlist == 4'hf ? 4'hd : zcmp_rlist - 4'h3;
wire       zcmp_rlist_invalid = zcmp_rlist < 4'h4 || (|EXTENSION_E && zcmp_rlist > 4'h6);

wire [11:0] zcmp_stack_adj_base =
	zcmp_rlist == 4'hf ? 12'h040 :
	zcmp_rlist >= 4'hc ? 12'h030 :
	zcmp_rlist >= 4'h8 ? 12'h020 : 12'h010;

wire [11:0] zcmp_stack_adj = zcmp_stack_adj_base + {6'h00, instr_in[3:2], 4'h0};

// Note we perform all load/stores before moving the stack pointer.
wire [11:0] zcmp_stack_lw_offset = -{6'h00, {zcmp_n_regs - uop_ctr}, 2'h0} + zcmp_stack_adj;
wire [11:0] zcmp_stack_sw_offset = -{6'h00, {zcmp_n_regs - uop_ctr}, 2'h0};

wire [4:0] zcmp_ls_reg =
	uop_ctr == 4'h0 ? 5'd01 : // ra
	uop_ctr == 4'h1 ? 5'd08 : // s0
	uop_ctr == 4'h2 ? 5'd09 : // s1
	5'd15 + {1'b0, uop_ctr};  // s2-s11 (s2 == x18)

wire [31:0] zcmp_push_sw_instr = 32'b00000000000000000010000000100011 | rfmt_rs1(5'd2) | rfmt_rs2(zcmp_ls_reg) | {
	zcmp_stack_sw_offset[11:5], 13'h0000, zcmp_stack_sw_offset[4:0], 7'h00
};

wire [31:0] zcmp_pop_lw_instr = 32'b00000000000000000010000000000011 | rfmt_rd(zcmp_ls_reg) | rfmt_rs1(5'd2)| {
	zcmp_stack_lw_offset[11:0], 20'h00000
};

wire [31:0] zcmp_push_stack_adj_instr = 32'b00000000000000000000000000010011 | rfmt_rd(5'd2) | rfmt_rs1(5'd2) | {
	-zcmp_stack_adj,
	20'h00000
};

wire [31:0] zcmp_pop_stack_adj_instr = 32'b00000000000000000000000000010011 | rfmt_rd(5'd2) | rfmt_rs1(5'd2) | {
	zcmp_stack_adj,
	20'h00000
};

wire [4:0] zcmp_sa01_r1s = {|instr_in[9:8], ~|instr_in[9:8], instr_in[9:7]};
wire [4:0] zcmp_sa01_r2s = {|instr_in[4:3], ~|instr_in[4:3], instr_in[4:2]};

wire       zcmp_sa01_invalid = |EXTENSION_E && |{instr_in[9:8], instr_in[4:3]};

// ----------------------------------------------------------------------------

generate
if (PASSTHROUGH) begin: instr_passthrough
	always @ (*) begin
		instr_is_32bit = 1'b1;
		instr_out = instr_in;
		invalid = 1'b0;
	end
end else begin: instr_decompress
	always @ (*) begin
		if (instr_in[1:0] == 2'b11) begin
			instr_is_32bit = 1'b1;
			instr_out = instr_in;
			invalid = 1'b0;
			in_uop_seq = 1'b0;
			uop_no_pc_update = 1'b0;
			uop_ctr_nxt_in_seq = uop_ctr;
		end else begin
			instr_is_32bit = 1'b0;
			instr_out = 32'd0;
			invalid = 1'b0;
			in_uop_seq = 1'b0;
			uop_no_pc_update = 1'b0;
			uop_ctr_nxt_in_seq = uop_ctr;
			casez (instr_in[15:0])
			16'b000???????????00: begin
				instr_out = 32'b00000000000000000000000000010011 | rfmt_rd(rd_s) | rfmt_rs1(5'd2)
					| {2'h0, instr_in[10:7], instr_in[12:11], instr_in[5], instr_in[6], 2'b00, 20'h00000};
				invalid   = ~|instr_in[12:2]; // Always-invalid all-zeroes instruction
			end
			16'b010???????????00:   instr_out = 32'b00000000000000000010000000000011 | rfmt_rd(rd_s) | rfmt_rs1(rs1_s)
				| {5'h00, instr_in[5], instr_in[12:10], instr_in[6], 2'b00, 20'h00000};
			16'b110???????????00:   instr_out = 32'b00000000000000000010000000100011 | rfmt_rs2(rs2_s) | rfmt_rs1(rs1_s)
				| {5'h00, instr_in[5], instr_in[12], 13'h0000, instr_in[11:10], instr_in[6], 2'b00, 7'h00};
			16'b000???????????01: instr_out = 32'b00000000000000000000000000010011 | rfmt_rd(rd_l) | rfmt_rs1(rs1_l) | imm_ci;
			16'b001???????????01:  instr_out = 32'b00000000000000000000000001101111  | rfmt_rd(5'd1) | imm_cj;
			16'b101???????????01:    instr_out = 32'b00000000000000000000000001101111  | rfmt_rd(5'd0) | imm_cj;
			16'b010???????????01:   instr_out = 32'b00000000000000000000000000010011 | rfmt_rd(rd_l) | imm_ci;
			16'b011???????????01: begin
				if (rd_l == 5'd2) begin
					// addi16sp
					instr_out = 32'b00000000000000000000000000010011 | rfmt_rd(5'd2) | rfmt_rs1(5'd2) |
						{{3{instr_in[12]}}, instr_in[4:3], instr_in[5], instr_in[2], instr_in[6], 24'h000000};
				end else begin
					instr_out = 32'b00000000000000000000000000110111 | rfmt_rd(rd_l) | {{15{instr_in[12]}}, instr_in[6:2], 12'h000};
				end
				invalid = ~|{instr_in[12], instr_in[6:2]}; // RESERVED if imm == 0
			end
			16'b0000??????????10: instr_out = 32'b00000000000000000001000000010011 | rfmt_rd(rs1_l) | rfmt_rs1(rs1_l) | imm_ci;
			16'b100001????????01: instr_out = 32'b01000000000000000101000000010011 | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | imm_ci;
			16'b100000????????01: instr_out = 32'b00000000000000000101000000010011 | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | imm_ci;
			16'b100?10????????01: instr_out = 32'b00000000000000000111000000010011 | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | imm_ci;
			16'b100011???11???01:  instr_out = 32'b00000000000000000111000000110011  | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | rfmt_rs2(rs2_s);
			16'b100011???10???01:   instr_out = 32'b00000000000000000110000000110011   | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | rfmt_rs2(rs2_s);
			16'b100011???01???01:  instr_out = 32'b00000000000000000100000000110011  | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | rfmt_rs2(rs2_s);
			16'b100011???00???01:  instr_out = 32'b01000000000000000000000000110011  | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | rfmt_rs2(rs2_s);
			16'b1001??????????10: begin
				if (|rs2_l) begin
					instr_out = 32'b00000000000000000000000000110011 | rfmt_rd(rd_l) | rfmt_rs1(rs1_l) | rfmt_rs2(rs2_l);
				end else if (|rs1_l) begin // jalr
					instr_out = 32'b00000000000000000000000001100111 | rfmt_rd(5'd1) | rfmt_rs1(rs1_l);
				end else begin // ebreak
					instr_out = 32'b00000000000100000000000001110011;
				end
			end
			16'b1000??????????10: begin
				if (|rs2_l) begin // mv
					instr_out = 32'b00000000000000000000000000110011 | rfmt_rd(rd_l) | rfmt_rs2(rs2_l);
				end else begin // jr
					instr_out = 32'b00000000000000000000000001100111 | rfmt_rs1(rs1_l);
					invalid = ~|rs1_l; // RESERVED
				end
			end
			16'b010???????????10: begin
				instr_out = 32'b00000000000000000010000000000011 | rfmt_rd(rd_l) | rfmt_rs1(5'd2) |
					{4'h0, instr_in[3:2], instr_in[12], instr_in[6:4], 2'b00, 20'h00000};
				invalid = ~|rd_l; // RESERVED
			end
			16'b110???????????10:    instr_out = 32'b00000000000000000010000000100011 | rfmt_rs2(rs2_l) | rfmt_rs1(5'd2)
				| {4'h0, instr_in[8:7], instr_in[12], 13'h0000, instr_in[11:9], 2'b00, 7'h00};
			16'b110???????????01:     instr_out = 32'b00000000000000000000000001100011 | rfmt_rs1(rs1_s) | imm_cb;
			16'b111???????????01:     instr_out = 32'b00000000000000000001000001100011 | rfmt_rs1(rs1_s) | imm_cb;

			// Optional Zcb instructions:
			16'b100000????????00: begin
				instr_out = 32'b00000000000000000100000000000011    | rfmt_rd(rd_s)  | rfmt_rs1(rs1_s) | imm_c_lb;
				invalid = ~|EXTENSION_ZCB;
			end
			16'b100001???0????00: begin
				instr_out = 32'b00000000000000000101000000000011    | rfmt_rd(rd_s)  | rfmt_rs1(rs1_s) | imm_c_lh;
				invalid = ~|EXTENSION_ZCB;
			end
			16'b100001???1????00: begin
				instr_out = 32'b00000000000000000001000000000011     | rfmt_rd(rd_s)  | rfmt_rs1(rs1_s) | imm_c_lh;
				invalid = ~|EXTENSION_ZCB;
			end
			16'b100010????????00: begin
				instr_out = 32'b00000000000000000000000000100011     | rfmt_rs2(rd_s) | rfmt_rs1(rs1_s) | imm_c_lb >> 13;
				invalid = ~|EXTENSION_ZCB;
			end
			16'b100011???0????00: begin
				instr_out = 32'b00000000000000000001000000100011     | rfmt_rs2(rd_s) | rfmt_rs1(rs1_s) | imm_c_lh >> 13;
				invalid = ~|EXTENSION_ZCB;
			end
			16'b100111???1100001: begin
				instr_out = 32'b00000000000000000111000000010011   | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | 32'h0ff00000;
				invalid = ~|EXTENSION_ZCB;
			end
			16'b100111???1100101: begin
				instr_out = 32'b01100000010000000001000000010011 | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s);
				invalid = ~|EXTENSION_ZCB || ~|EXTENSION_ZBB;
			end
			16'b100111???1101001: begin
				instr_out = 32'b00001000000000000100000000110011 | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s);
				invalid = ~|EXTENSION_ZCB || ~|EXTENSION_ZBB;
			end
			16'b100111???1101101: begin
				instr_out = 32'b01100000010100000001000000010011 | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s);
				invalid = ~|EXTENSION_ZCB || ~|EXTENSION_ZBB;
			end
			16'b100111???1110101: begin
				instr_out = 32'b00000000000000000100000000010011   | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | 32'hfff00000;
				invalid = ~|EXTENSION_ZCB;
			end
			16'b100111???10???01: begin
				instr_out = 32'b00000010000000000000000000110011    | rfmt_rd(rs1_s) | rfmt_rs1(rs1_s) | rfmt_rs2(rs2_s);
				invalid = ~|EXTENSION_ZCB || ~|EXTENSION_M;
			end

			// Optional Zclsd instructions:
			16'b011??????????000: begin
				instr_out = 32'b00000000000000000011000000000011 | rfmt_rd(rd_s) | rfmt_rs1(rs1_s)
					| {4'h0, instr_in[6:5], instr_in[12:10], 3'b000, 20'h00000};
				invalid = ~|EXTENSION_ZILSD || ~|EXTENSION_ZCLSD;
			end
			16'b111??????????000: begin
				instr_out = 32'b00000000000000000011000000100011 | rfmt_rs2(rs2_s) | rfmt_rs1(rs1_s)
					| {4'h0, instr_in[6:5], instr_in[12], 13'h0000, instr_in[11:10], 3'b000, 7'h00};
				invalid = ~|EXTENSION_ZILSD || ~|EXTENSION_ZCLSD;
			end
			16'b011?????0?????10: begin
				instr_out = 32'b00000000000000000011000000000011 | rfmt_rd(rd_l) | rfmt_rs1(5'd2) |
					{3'h0, instr_in[4:2], instr_in[12], instr_in[6:5], 3'b000, 20'h00000};
				invalid = ~|EXTENSION_ZILSD || ~|EXTENSION_ZCLSD || ~|rd_l; // RESERVED
			end
			16'b111??????????010: begin
				instr_out = 32'b00000000000000000011000000100011 | rfmt_rs2(rs2_l) | rfmt_rs1(5'd2)
					| {3'h0, instr_in[9:7], instr_in[12], 13'h0000, instr_in[11:10], 3'b000, 7'h00};
				invalid = ~|EXTENSION_ZILSD || ~|EXTENSION_ZCLSD;
			end

			// Optional Zcmp instructions:
			16'b10111000??????10: if (~|EXTENSION_ZCMP || zcmp_rlist_invalid) begin
				invalid = 1'b1;
			end else if (uop_ctr == 4'hf) begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = 4'h0;
				instr_out = zcmp_push_stack_adj_instr;
			end else begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = uop_ctr + 4'h1;
				instr_out = zcmp_push_sw_instr;
				uop_no_pc_update = 1'b1;
				if (uop_ctr_nxt_in_seq == zcmp_n_regs) begin
					uop_ctr_nxt_in_seq = 4'hf;
				end
			end

			16'b10111010??????10: if (~|EXTENSION_ZCMP || zcmp_rlist_invalid) begin
				invalid = 1'b1;
			end else if (uop_ctr == 4'hf) begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = 4'h0;
				instr_out = zcmp_pop_stack_adj_instr;
			end else begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = uop_ctr + 4'h1;
				uop_no_pc_update = 1'b1;
				instr_out = zcmp_pop_lw_instr;
				if (uop_ctr_nxt_in_seq == zcmp_n_regs) begin
					uop_ctr_nxt_in_seq = 4'hf;
				end
			end

			16'b10111110??????10: if (~|EXTENSION_ZCMP || zcmp_rlist_invalid) begin
				invalid = 1'b1;
			end else if (uop_ctr == 4'he) begin
				// Note although this is only the first instruction in the uninterruptible sequence,
				// we mark this instruction as uninterruptible: there is some special case logic to
				// allow this jump to execute without flushing the final stack adjust uop, which can
				// cause the wrong exception PC to be sampled if this uop is interrupted.
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = uop_ctr + 4'h1;
				instr_out = 32'b00000000000000000000000001100111 | rfmt_rs1(5'd1);
			end else if (uop_ctr == 4'hf) begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = 4'h0;
				uop_no_pc_update = 1'b1;
				instr_out = zcmp_pop_stack_adj_instr;
			end else begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = uop_ctr + 4'h1;
				instr_out = zcmp_pop_lw_instr;
				uop_no_pc_update = 1'b1;
				if (uop_ctr_nxt_in_seq == zcmp_n_regs) begin
					uop_ctr_nxt_in_seq = 4'he;
				end
			end

			16'b10111100??????10: if (~|EXTENSION_ZCMP || zcmp_rlist_invalid) begin
				invalid = 1'b1;
			end else if (uop_ctr == 4'hd) begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = uop_ctr + 4'h1;
				uop_no_pc_update = 1'b1;
				instr_out = 32'b00000000000000000000000000010011 | rfmt_rd(5'd10); // li a0, 0
			end else if (uop_ctr == 4'he) begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = uop_ctr + 4'h1;
				instr_out = 32'b00000000000000000000000001100111 | rfmt_rs1(5'd1);
			end else if (uop_ctr == 4'hf) begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = 4'h0;
				uop_no_pc_update = 1'b1;
				instr_out = zcmp_pop_stack_adj_instr;
			end else begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = uop_ctr + 4'h1;
				uop_no_pc_update = 1'b1;
				instr_out = zcmp_pop_lw_instr;
				if (uop_ctr_nxt_in_seq == zcmp_n_regs) begin
					uop_ctr_nxt_in_seq = 4'hd;
				end
			end

			16'b101011???01???10: if (~|EXTENSION_ZCMP || zcmp_sa01_invalid) begin
				invalid = 1'b1;
			end else if (uop_ctr == 4'h0) begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = uop_ctr + 4'h1;
				uop_no_pc_update = 1'b1;
				instr_out = 32'b00000000000000000000000000010011 | rfmt_rd(zcmp_sa01_r1s) | rfmt_rs1(5'd10);
			end else begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = 4'h0;
				instr_out = 32'b00000000000000000000000000010011 | rfmt_rd(zcmp_sa01_r2s) | rfmt_rs1(5'd11);
			end

			16'b101011???11???10: if (~|EXTENSION_ZCMP || zcmp_sa01_invalid) begin
				invalid = 1'b1;
			end else if (uop_ctr == 4'h0) begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = uop_ctr + 4'h1;
				uop_no_pc_update = 1'b1;
				instr_out = 32'b00000000000000000000000000010011 | rfmt_rd(5'd10) | rfmt_rs1(zcmp_sa01_r1s);
			end else begin
				in_uop_seq = 1'b1;
				uop_ctr_nxt_in_seq = 4'h0;
				instr_out = 32'b00000000000000000000000000010011 | rfmt_rd(5'd11) | rfmt_rs1(zcmp_sa01_r2s);
			end

			default: invalid = 1'b1;
			endcase
		end
	end
end
endgenerate

generate
if (EXTENSION_ZCMP) begin: have_uop_ctr
	reg [3:0] uop_ctr_r;
	assign uop_ctr = uop_ctr_r;
	always @ (posedge clk or negedge rst_n) begin
		if (!rst_n) begin
			uop_ctr_r <= 4'h0;
		end else begin
			uop_ctr_r <= uop_ctr_nxt;










		end
	end
end else begin: no_uop_ctr
	assign uop_ctr = 4'h0;
end
endgenerate

endmodule


`default_nettype wire

/*****************************************************************************\
|                        Copyright (C) 2022 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

`default_nettype none

// Hazard3 interrupt controller. Support for up to 512 external interrupt
// lines, with up to 16 levels of preemption.

module hazard3_irq_ctrl #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
) (
	input  wire                clk,
	input  wire                clk_always_on,
	input  wire                rst_n,

	// CSR interface
	input  wire [11:0]         addr,
	input  wire [1:0]          wtype,
	input  wire                wen_m_mode,
	input  wire                ren_m_mode,
	input  wire [W_DATA-1:0]   wdata_raw,
	input  wire [W_DATA-1:0]   wdata,
	output reg  [W_DATA-1:0]   rdata,

	// Trap entry/exit signals for context update
	input  wire                trapreg_update_enter,
	input  wire                trapreg_update_exit,
	input  wire                trap_entry_is_eirq,

	// Interface for clearing and saving mie.mtie/msie via meicontext
	output wire                meicontext_clearts,
	input  wire                mie_mtie,
	input  wire                mie_msie,

	// External IRQ inputs:
	input  wire [NUM_IRQS-1:0] irq,

	// mip.meip:
	output wire                external_irq_pending
);

/*****************************************************************************\
|                        Copyright (C) 2021 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// ALU operation selectors

localparam ALUOP_ADD     = 6'h00; 
localparam ALUOP_SUB     = 6'h01; 
localparam ALUOP_LT      = 6'h02;
localparam ALUOP_LTU     = 6'h04;
localparam ALUOP_AND     = 6'h06;
localparam ALUOP_OR      = 6'h07;
localparam ALUOP_XOR     = 6'h08;
localparam ALUOP_SRL     = 6'h09;
localparam ALUOP_SRA     = 6'h0a;
localparam ALUOP_SLL     = 6'h0b;
localparam ALUOP_MULDIV  = 6'h0c;
localparam ALUOP_RS2     = 6'h0d; // differs from AND/OR/XOR in [1:0]
// Bitmanip ALU operations (some also used by AMOs):
localparam ALUOP_SHXADD  = 6'h20;
localparam ALUOP_CLZ     = 6'h23;
localparam ALUOP_CPOP    = 6'h24;
localparam ALUOP_CTZ     = 6'h25;
localparam ALUOP_ANDN    = 6'h26; // Same LSBs as non-inverted
localparam ALUOP_ORN     = 6'h27; // Same LSBs as non-inverted
localparam ALUOP_XNOR    = 6'h28; // Same LSBs as non-inverted
localparam ALUOP_MAX     = 6'h29;
localparam ALUOP_MAXU    = 6'h2a;
localparam ALUOP_MIN     = 6'h2b;
localparam ALUOP_MINU    = 6'h2c;
localparam ALUOP_ORC_B   = 6'h2d;
localparam ALUOP_REV8    = 6'h2e;
localparam ALUOP_ROL     = 6'h2f;
localparam ALUOP_ROR     = 6'h30;
localparam ALUOP_SEXT_B  = 6'h31;
localparam ALUOP_SEXT_H  = 6'h32;
localparam ALUOP_ZEXT_H  = 6'h33;

localparam ALUOP_CLMUL   = 6'h34;

localparam ALUOP_BCLR    = 6'h35;
localparam ALUOP_BEXT    = 6'h36;
localparam ALUOP_BINV    = 6'h37;
localparam ALUOP_BSET    = 6'h38;

localparam ALUOP_PACK    = 6'h39;
localparam ALUOP_PACKH   = 6'h3a;
localparam ALUOP_BREV8   = 6'h3b;
localparam ALUOP_ZIP     = 6'h3c;
localparam ALUOP_UNZIP   = 6'h3d;

localparam ALUOP_BEXTM   = 6'h3e;

localparam ALUOP_XPERM   = 6'h3f;

// Parameters to control ALU input muxes. Bypass mux paths are
// controlled by X, so D has no parameters to choose these.

localparam ALUSRCA_RS1 = 1'h0;
localparam ALUSRCA_PC  = 1'h1;

localparam ALUSRCB_RS2 = 1'h0;
localparam ALUSRCB_IMM = 1'h1;

localparam MEMOP_LW   = 5'h00;
localparam MEMOP_LH   = 5'h01;
localparam MEMOP_LB   = 5'h02;
localparam MEMOP_LHU  = 5'h03;
localparam MEMOP_LBU  = 5'h04;
localparam MEMOP_SW   = 5'h05;
localparam MEMOP_SH   = 5'h06;
localparam MEMOP_SB   = 5'h07;

localparam MEMOP_LR_W = 5'h08;
localparam MEMOP_SC_W = 5'h09;
localparam MEMOP_AMO  = 5'h0a;
localparam MEMOP_NONE = 5'h10;

localparam BCOND_NEVER  = 2'h0;
localparam BCOND_ALWAYS = 2'h1;
localparam BCOND_ZERO   = 2'h2;
localparam BCOND_NZERO  = 2'h3;

// CSR access types

localparam CSR_WTYPE_W    = 2'h0;
localparam CSR_WTYPE_S    = 2'h1;
localparam CSR_WTYPE_C    = 2'h2;

// Exceptional condition signals which travel alongside (or instead of)
// instructions in the pipeline. These are speculative and can be flushed on
// e.g. branch mispredict. These mostly align with mcause values.

localparam EXCEPT_NONE           = 4'hf;

localparam EXCEPT_INSTR_MISALIGN = 4'h0;
localparam EXCEPT_INSTR_FAULT    = 4'h1;
localparam EXCEPT_INSTR_ILLEGAL  = 4'h2;
localparam EXCEPT_EBREAK         = 4'h3;
localparam EXCEPT_LOAD_ALIGN     = 4'h4;
localparam EXCEPT_LOAD_FAULT     = 4'h5;
localparam EXCEPT_STORE_ALIGN    = 4'h6;
localparam EXCEPT_STORE_FAULT    = 4'h7;
localparam EXCEPT_ECALL_U        = 4'h8;
// MRET, Return from M-mode: not really an exception, but handled like one
localparam EXCEPT_MRET           = 4'ha;
localparam EXCEPT_ECALL_M        = 4'hb;
// spare: c
// spare: d
// REFETCH: flush and refetch sequentially-following instructions, e.g. on
// executing fence.i. Jumps from stage 3 to get ordering against L/S dphase.
localparam EXCEPT_REFETCH        = 4'he;

// Operations for M extension (these are just instr[14:12])

localparam M_OP_MUL    = 3'h0;
localparam M_OP_MULH   = 3'h1;
localparam M_OP_MULHSU = 3'h2;
localparam M_OP_MULHU  = 3'h3;
localparam M_OP_DIV    = 3'h4;
localparam M_OP_DIVU   = 3'h5;
localparam M_OP_REM    = 3'h6;
localparam M_OP_REMU   = 3'h7;
/*****************************************************************************\
|                        Copyright (C) 2022 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// List of addresses for CSRs implemented by Hazard3, including custom CSRs.

// ----------------------------------------------------------------------------
// M-mode CSRs

// Machine Information Registers (RO)
localparam MVENDORID      = 12'hf11; // Vendor ID.
localparam MARCHID        = 12'hf12; // Architecture ID.
localparam MIMPID         = 12'hf13; // Implementation ID.
localparam MHARTID        = 12'hf14; // Hardware thread ID.
localparam MCONFIGPTR     = 12'hf15; // Pointer to configuration data structure.

// Machine Trap Setup (RW)
localparam MSTATUS        = 12'h300; // Machine status register.
localparam MSTATUSH       = 12'h310; // As of priv-1.12 this must be present even if tied 0.
localparam MISA           = 12'h301; // ISA and extensions
localparam MEDELEG        = 12'h302; // Machine exception delegation register.
localparam MIDELEG        = 12'h303; // Machine interrupt delegation register.
localparam MIE            = 12'h304; // Machine interrupt-enable register.
localparam MTVEC          = 12'h305; // Machine trap-handler base address.
localparam MCOUNTEREN     = 12'h306; // Machine counter enable.

// Machine Trap Handling (RW)
localparam MSCRATCH       = 12'h340; // Scratch register for machine trap handlers.
localparam MEPC           = 12'h341; // Machine exception program counter.
localparam MCAUSE         = 12'h342; // Machine trap cause.
localparam MTVAL          = 12'h343; // Machine bad address or instruction.
localparam MIP            = 12'h344; // Machine interrupt pending.

// Machine Memory Protection (RW)
localparam PMPCFG0        = 12'h3a0; // Physical memory protection configuration.
localparam PMPCFG1        = 12'h3a1; // Physical memory protection configuration, RV32 only.
localparam PMPCFG2        = 12'h3a2; // Physical memory protection configuration.
localparam PMPCFG3        = 12'h3a3; // Physical memory protection configuration, RV32 only.
localparam PMPADDR0       = 12'h3b0; // Physical memory protection address register.
localparam PMPADDR1       = 12'h3b1; // ...
localparam PMPADDR2       = 12'h3b2;
localparam PMPADDR3       = 12'h3b3;
localparam PMPADDR4       = 12'h3b4;
localparam PMPADDR5       = 12'h3b5;
localparam PMPADDR6       = 12'h3b6;
localparam PMPADDR7       = 12'h3b7;
localparam PMPADDR8       = 12'h3b8;
localparam PMPADDR9       = 12'h3b9;
localparam PMPADDR10      = 12'h3ba;
localparam PMPADDR11      = 12'h3bb;
localparam PMPADDR12      = 12'h3bc;
localparam PMPADDR13      = 12'h3bd;
localparam PMPADDR14      = 12'h3be;
localparam PMPADDR15      = 12'h3bf;

localparam MSECCFG        = 12'h747;
localparam MSECCFGH       = 12'h757;

// Performance counters (RW)
localparam MCYCLE         = 12'hb00; // Raw cycles since start of day
localparam MINSTRET       = 12'hb02; // Instruction retire count since start of day
localparam MHPMCOUNTER3   = 12'hb03; // WARL (we tie to 0)
localparam MHPMCOUNTER4   = 12'hb04; // WARL (we tie to 0)
localparam MHPMCOUNTER5   = 12'hb05; // WARL (we tie to 0)
localparam MHPMCOUNTER6   = 12'hb06; // WARL (we tie to 0)
localparam MHPMCOUNTER7   = 12'hb07; // WARL (we tie to 0)
localparam MHPMCOUNTER8   = 12'hb08; // WARL (we tie to 0)
localparam MHPMCOUNTER9   = 12'hb09; // WARL (we tie to 0)
localparam MHPMCOUNTER10  = 12'hb0a; // WARL (we tie to 0)
localparam MHPMCOUNTER11  = 12'hb0b; // WARL (we tie to 0)
localparam MHPMCOUNTER12  = 12'hb0c; // WARL (we tie to 0)
localparam MHPMCOUNTER13  = 12'hb0d; // WARL (we tie to 0)
localparam MHPMCOUNTER14  = 12'hb0e; // WARL (we tie to 0)
localparam MHPMCOUNTER15  = 12'hb0f; // WARL (we tie to 0)
localparam MHPMCOUNTER16  = 12'hb10; // WARL (we tie to 0)
localparam MHPMCOUNTER17  = 12'hb11; // WARL (we tie to 0)
localparam MHPMCOUNTER18  = 12'hb12; // WARL (we tie to 0)
localparam MHPMCOUNTER19  = 12'hb13; // WARL (we tie to 0)
localparam MHPMCOUNTER20  = 12'hb14; // WARL (we tie to 0)
localparam MHPMCOUNTER21  = 12'hb15; // WARL (we tie to 0)
localparam MHPMCOUNTER22  = 12'hb16; // WARL (we tie to 0)
localparam MHPMCOUNTER23  = 12'hb17; // WARL (we tie to 0)
localparam MHPMCOUNTER24  = 12'hb18; // WARL (we tie to 0)
localparam MHPMCOUNTER25  = 12'hb19; // WARL (we tie to 0)
localparam MHPMCOUNTER26  = 12'hb1a; // WARL (we tie to 0)
localparam MHPMCOUNTER27  = 12'hb1b; // WARL (we tie to 0)
localparam MHPMCOUNTER28  = 12'hb1c; // WARL (we tie to 0)
localparam MHPMCOUNTER29  = 12'hb1d; // WARL (we tie to 0)
localparam MHPMCOUNTER30  = 12'hb1e; // WARL (we tie to 0)
localparam MHPMCOUNTER31  = 12'hb1f; // WARL (we tie to 0)

localparam MCYCLEH        = 12'hb80; // High halves of each counter
localparam MINSTRETH      = 12'hb82;
localparam MHPMCOUNTER3H  = 12'hb83;
localparam MHPMCOUNTER4H  = 12'hb84;
localparam MHPMCOUNTER5H  = 12'hb85;
localparam MHPMCOUNTER6H  = 12'hb86;
localparam MHPMCOUNTER7H  = 12'hb87;
localparam MHPMCOUNTER8H  = 12'hb88;
localparam MHPMCOUNTER9H  = 12'hb89;
localparam MHPMCOUNTER10H = 12'hb8a;
localparam MHPMCOUNTER11H = 12'hb8b;
localparam MHPMCOUNTER12H = 12'hb8c;
localparam MHPMCOUNTER13H = 12'hb8d;
localparam MHPMCOUNTER14H = 12'hb8e;
localparam MHPMCOUNTER15H = 12'hb8f;
localparam MHPMCOUNTER16H = 12'hb90;
localparam MHPMCOUNTER17H = 12'hb91;
localparam MHPMCOUNTER18H = 12'hb92;
localparam MHPMCOUNTER19H = 12'hb93;
localparam MHPMCOUNTER20H = 12'hb94;
localparam MHPMCOUNTER21H = 12'hb95;
localparam MHPMCOUNTER22H = 12'hb96;
localparam MHPMCOUNTER23H = 12'hb97;
localparam MHPMCOUNTER24H = 12'hb98;
localparam MHPMCOUNTER25H = 12'hb99;
localparam MHPMCOUNTER26H = 12'hb9a;
localparam MHPMCOUNTER27H = 12'hb9b;
localparam MHPMCOUNTER28H = 12'hb9c;
localparam MHPMCOUNTER29H = 12'hb9d;
localparam MHPMCOUNTER30H = 12'hb9e;
localparam MHPMCOUNTER31H = 12'hb9f;

localparam MCOUNTINHIBIT  = 12'h320; // Count inhibit register for mcycle/minstret
localparam MHPMEVENT3     = 12'h323; // WARL (we tie to 0)
localparam MHPMEVENT4     = 12'h324; // WARL (we tie to 0)
localparam MHPMEVENT5     = 12'h325; // WARL (we tie to 0)
localparam MHPMEVENT6     = 12'h326; // WARL (we tie to 0)
localparam MHPMEVENT7     = 12'h327; // WARL (we tie to 0)
localparam MHPMEVENT8     = 12'h328; // WARL (we tie to 0)
localparam MHPMEVENT9     = 12'h329; // WARL (we tie to 0)
localparam MHPMEVENT10    = 12'h32a; // WARL (we tie to 0)
localparam MHPMEVENT11    = 12'h32b; // WARL (we tie to 0)
localparam MHPMEVENT12    = 12'h32c; // WARL (we tie to 0)
localparam MHPMEVENT13    = 12'h32d; // WARL (we tie to 0)
localparam MHPMEVENT14    = 12'h32e; // WARL (we tie to 0)
localparam MHPMEVENT15    = 12'h32f; // WARL (we tie to 0)
localparam MHPMEVENT16    = 12'h330; // WARL (we tie to 0)
localparam MHPMEVENT17    = 12'h331; // WARL (we tie to 0)
localparam MHPMEVENT18    = 12'h332; // WARL (we tie to 0)
localparam MHPMEVENT19    = 12'h333; // WARL (we tie to 0)
localparam MHPMEVENT20    = 12'h334; // WARL (we tie to 0)
localparam MHPMEVENT21    = 12'h335; // WARL (we tie to 0)
localparam MHPMEVENT22    = 12'h336; // WARL (we tie to 0)
localparam MHPMEVENT23    = 12'h337; // WARL (we tie to 0)
localparam MHPMEVENT24    = 12'h338; // WARL (we tie to 0)
localparam MHPMEVENT25    = 12'h339; // WARL (we tie to 0)
localparam MHPMEVENT26    = 12'h33a; // WARL (we tie to 0)
localparam MHPMEVENT27    = 12'h33b; // WARL (we tie to 0)
localparam MHPMEVENT28    = 12'h33c; // WARL (we tie to 0)
localparam MHPMEVENT29    = 12'h33d; // WARL (we tie to 0)
localparam MHPMEVENT30    = 12'h33e; // WARL (we tie to 0)
localparam MHPMEVENT31    = 12'h33f; // WARL (we tie to 0)

// Other standard M-mode CSRs:
localparam MENVCFG        = 12'h30a;
localparam MENVCFGH       = 12'h31a;

// Custom M-mode CSRs:
localparam PMPCFGM0       = 12'hbd0; // Make PMP regions M-mode without locking
//                              bd1  // (reserved for >32 regions)

localparam MEIEA          = 12'hbe0; // External interrupt pending array
localparam MEIPA          = 12'hbe1; // External interrupt enable array
localparam MEIFA          = 12'hbe2; // External interrupt force array
localparam MEIPRA         = 12'hbe3; // External interrupt priority array
localparam MEINEXT        = 12'hbe4; // Next external interrupt
localparam MEICONTEXT     = 12'hbe5; // External interrupt context register

localparam MSLEEP         = 12'hbf0; // M-mode sleep control register
localparam H3MISA         = 12'hbf1; // Hazard3 M-mode ISA identification register

// ----------------------------------------------------------------------------
// U-mode CSRs

// Read-only aliases of M-mode counter CSRs:
localparam CYCLE          = 12'hc00;
localparam TIME           = 12'hc01;
localparam INSTRET        = 12'hc02;
localparam CYCLEH         = 12'hc80;
localparam TIMEH          = 12'hc81;
localparam INSTRETH       = 12'hc82;

// Custom U-mode CSRs
localparam SLEEP          = 12'h8f0; // U-mode subset of M-mode sleep control

// ----------------------------------------------------------------------------
// Trigger Module

localparam TSELECT       = 12'h7a0;
localparam TDATA1        = 12'h7a1;
localparam TDATA2        = 12'h7a2;
localparam TDATA3        = 12'h7a3;
localparam TINFO         = 12'h7a4;
localparam TCONTROL      = 12'h7a5;
localparam MCONTEXT      = 12'h7a8;

// ----------------------------------------------------------------------------
// D-mode CSRs

localparam DCSR           = 12'h7b0;
localparam DPC            = 12'h7b1;
localparam DMDATA0        = 12'hbff; // Custom read/write

localparam MAX_IRQS = 512;
localparam [3:0] IRQ_PRIORITY_MASK = ~(4'hf >> IRQ_PRIORITY_BITS);
localparam W_IRQ_INDEX = $clog2(MAX_IRQS);

// ----------------------------------------------------------------------------
// IRQ input flops

// Register external IRQ signals (mainly to avoid a through-path from IRQs to
// bus request signals). Always clocked, as it's used to generate a wakeup.
// Input registers can be removed on a per-IRQ basis, but this should be done
// with care as it does create a through-path from the IRQ to the bus.

wire [NUM_IRQS-1:0] irq_r;

genvar g;
generate
for (g = 0; g < NUM_IRQS; g = g + 1) begin: irq_reg_loop
	if (IRQ_INPUT_BYPASS[g]) begin: no_reg
		assign irq_r[g] = irq[g];
	end else begin: have_reg
		reg q;
		always @ (posedge clk_always_on or negedge rst_n) begin
			if (!rst_n) begin
				q <= 1'b0;
			end else begin
				q <= irq[g];
			end
		end
		assign irq_r[g] = q;
	end
end
endgenerate

// ----------------------------------------------------------------------------
// CSR write

// Assigned later:
wire [W_IRQ_INDEX-1:0] meinext_irq;
wire                   meinext_noirq;
reg  [3:0]             eirq_highest_priority;

// Interrupt array registers:
reg  [NUM_IRQS-1:0]   meiea;
reg  [NUM_IRQS-1:0]   meifa;
reg  [4*NUM_IRQS-1:0] meipra;

// Padded vectors for CSR readout
wire [MAX_IRQS-1:0]   meiea_rdata  = {{MAX_IRQS-NUM_IRQS{1'b0}}, meiea};
wire [MAX_IRQS-1:0]   meifa_rdata  = {{MAX_IRQS-NUM_IRQS{1'b0}}, meifa};
wire [4*MAX_IRQS-1:0] meipra_rdata = {{4*(MAX_IRQS-NUM_IRQS){1'b0}}, meipra};

always @ (posedge clk or negedge rst_n) begin: update_irq_reg_arrays
	reg signed [31:0] i;
	if (!rst_n) begin
		meiea <= {NUM_IRQS{1'b0}};
		meifa <= {NUM_IRQS{1'b0}};
		meipra <= {4*NUM_IRQS{1'b0}};
	end else begin
		for (i = 0; i < NUM_IRQS; i = i + 1) begin
			// CSR write update. Note raw wdata is used for array indexing --
			// necessary for correctness, and also avoid a loop with rdata.
			if (wen_m_mode && addr == MEIEA  && $signed(wdata_raw[4:0]) == i[W_IRQ_INDEX-1:4]) begin
				meiea[i]  <= wdata[16 + (i % 16)];
			end
			if (wen_m_mode && addr == MEIFA  && $signed(wdata_raw[4:0]) == i[W_IRQ_INDEX-1:4]) begin
				meifa[i]  <= wdata[16 + (i % 16)];
			end
			if (wen_m_mode && addr == MEIPRA && $signed(wdata_raw[6:0]) == i[W_IRQ_INDEX-1:2]) begin
				meipra[4 * i +: 4] <= wdata[16 + 4 * (i % 4) +: 4] & IRQ_PRIORITY_MASK;
			end
			// Clear IRQ force when the corresponding IRQ is sampled from meinext
			// (so that an IRQ can be posted *once* without modifying the ISR source)
			if (meinext_irq == i[W_IRQ_INDEX-1:0] && ren_m_mode && addr == MEINEXT && !meinext_noirq) begin
				meifa[i[$clog2(NUM_IRQS)-1:0]] <= 1'b0;
			end
		end
	end
end

reg [3:0]             meicontext_pppreempt;
reg [3:0]             meicontext_ppreempt;
reg [4:0]             meicontext_preempt;
reg                   meicontext_noirq;
reg [W_IRQ_INDEX-1:0] meicontext_irq;
reg                   meicontext_mreteirq;

wire [4:0] preempt_level_next = meinext_noirq ? 5'h10 : (
	(5'd1 << (4 - IRQ_PRIORITY_BITS)) + {1'b0, eirq_highest_priority}
);

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		meicontext_pppreempt <= 4'h0;
		meicontext_ppreempt <= 4'h0;
		meicontext_preempt <= 5'h0;
		meicontext_noirq <= 1'b1;
		meicontext_irq <= {W_IRQ_INDEX{1'b0}};
		meicontext_mreteirq <= 1'b0;
	end else if (trapreg_update_enter) begin
		if (trap_entry_is_eirq) begin
			// Priority save. Note the MSB of preempt needn't be saved since,
			// when it is set, preemption is impossible, so we won't be here.
			meicontext_pppreempt <= meicontext_ppreempt & IRQ_PRIORITY_MASK;
			meicontext_ppreempt <= meicontext_preempt[3:0] & IRQ_PRIORITY_MASK;
			// Setting preempt isn't strictly necessary, since an updating read
			// of meinext ought to be performed before re-enabling IRQs via
			// mstatus.mie, but it seems the least surprising thing to do:
			meicontext_preempt <= preempt_level_next & {1'b1, IRQ_PRIORITY_MASK};
			meicontext_mreteirq <= 1'b1;
		end else begin
			meicontext_mreteirq <= 1'b0;
		end
	end else if (trapreg_update_exit) begin
		meicontext_mreteirq <= 1'b0;
		if (meicontext_mreteirq) begin
			// Priority restore
			meicontext_pppreempt <= 4'h0;
			meicontext_ppreempt <= meicontext_pppreempt & IRQ_PRIORITY_MASK;
			meicontext_preempt <= {1'b0, meicontext_ppreempt & IRQ_PRIORITY_MASK};
		end
	end else if (wen_m_mode && addr == MEICONTEXT) begin
		meicontext_pppreempt <= wdata[31:28] & IRQ_PRIORITY_MASK;
		meicontext_ppreempt <= wdata[27:24] & IRQ_PRIORITY_MASK;
		meicontext_preempt <= wdata[20:16] & {1'b1, IRQ_PRIORITY_MASK};
		meicontext_noirq <= wdata[15];
		meicontext_irq <= wdata[12:4];
		meicontext_mreteirq <= wdata[0];
	end else if (wen_m_mode && addr == MEINEXT && wdata[0]) begin
		// Interrupt has been sampled, with the update request set, so update
		// the context (including preemption level) appropriately.
		meicontext_preempt <= preempt_level_next & {1'b1, IRQ_PRIORITY_MASK};
		meicontext_noirq <= meinext_noirq;
		meicontext_irq <= meinext_irq;
	end
end

assign meicontext_clearts = wen_m_mode && wtype != CSR_WTYPE_C && addr == MEICONTEXT && wdata_raw[1];

// ----------------------------------------------------------------------------
// External interrupt logic

// Trap request is asserted when there is an interrupt at or above our current
// preemption level. meinext displays interrupts at or above our *previous*
// preemption level: this masking helps avoid re-taking IRQs in frames that you
// have preempted. 

wire [NUM_IRQS-1:0] meipa = irq_r | meifa;
wire [MAX_IRQS-1:0] meipa_rdata = {{MAX_IRQS-NUM_IRQS{1'b0}}, meipa};

reg [NUM_IRQS-1:0] eirq_active_above_preempt;
reg [NUM_IRQS-1:0] eirq_active_above_ppreempt;

always @ (*) begin: eirq_compare
	integer i;
	for (i = 0; i < NUM_IRQS; i = i + 1) begin
		eirq_active_above_preempt[i]  = meipa[i] && meiea[i] && {1'b0, meipra[i * 4 +: 4]} >= meicontext_preempt;
		eirq_active_above_ppreempt[i] = meipa[i] && meiea[i] &&        meipra[i * 4 +: 4]  >= meicontext_ppreempt;
	end
end

assign external_irq_pending =  |eirq_active_above_preempt;
assign meinext_noirq        = ~|eirq_active_above_ppreempt;

// Two things remaining to calculate:
//
// - What is the IRQ number of the highest-priority pending IRQ that is above
//   meicontext.ppreempt
// - What is the priority of that IRQ
//
// In the second case we can relax the calculation to ignore ppreempt, since it
// only needs to be valid if such an IRQ exists. Currently we choose to reuse
// the same priority selector (possibly longer critpath while saving area), but
// we could use a second priority selector that ignores ppreempt masking.

wire [NUM_IRQS-1:0]    highest_eirq_onehot;
wire [W_IRQ_INDEX-1:0] meinext_irq_unmasked;

hazard3_onehot_priority_dynamic #(
	.W_REQ                 (NUM_IRQS),
	.N_PRIORITIES          (16),
	.PRIORITY_HIGHEST_WINS (1),
	.TIEBREAK_HIGHEST_WINS (0)
) eirq_priority_u (
	.pri (meipra[4*NUM_IRQS-1:0] & {NUM_IRQS{IRQ_PRIORITY_MASK}}),
	.req (eirq_active_above_ppreempt),
	.gnt (highest_eirq_onehot)
);

always @ (*) begin: get_highest_eirq_priority
	integer i;
	eirq_highest_priority = 4'h0;
	for (i = 0; i < NUM_IRQS; i = i + 1) begin
		eirq_highest_priority = eirq_highest_priority | (
			meipra[4 * i +: 4] & {4{highest_eirq_onehot[i]}}
		);
	end
end

wire [$clog2(NUM_IRQS)-1:0] meinext_irq_unmasked_nopad;

hazard3_onehot_encode #(
	.W_REQ (NUM_IRQS)
) eirq_encode_u (
	.req (highest_eirq_onehot),
	.gnt (meinext_irq_unmasked_nopad)
);

generate
if ($clog2(NUM_IRQS) == $clog2(MAX_IRQS)) begin: encode_eirq_no_padding
	assign meinext_irq_unmasked = meinext_irq_unmasked_nopad;
end else begin: encode_eirq_padded
	assign meinext_irq_unmasked = {
		{$clog2(MAX_IRQS) - $clog2(NUM_IRQS){1'b0}},
		meinext_irq_unmasked_nopad
	};
end
endgenerate

// It is unnecessary to mask meinext_irq based on meinext_noirq because:
// - The value of the CSR field is unimportant when noirq is set
// - There are no IRQ inputs to the priority selector when there
//   are no IRQs, so result is already 0.
assign meinext_irq = meinext_irq_unmasked;

// ----------------------------------------------------------------------------
// CSR read

always @ (*) begin
	rdata = {W_DATA{1'b0}};
	case (addr)

	MEIEA: rdata = {
		meiea_rdata[wdata_raw[4:0] * 16 +: 16],
		16'h0
	};

	MEIPA: rdata = {
		meipa_rdata[wdata_raw[4:0] * 16 +: 16],
		16'h0
	};

	MEIFA: rdata = {
		meifa_rdata[wdata_raw[4:0] * 16 +: 16],
		16'h0
	};

	MEIPRA: rdata = {
		meipra_rdata[wdata_raw[6:0] * 16 +: 16],
		16'h0
	};

	MEINEXT: rdata = {
		meinext_noirq,
		20'h0,
		meinext_irq,
		2'h0
	};

	MEICONTEXT: rdata = {
		meicontext_pppreempt,
		meicontext_ppreempt,
		3'h0,
		meicontext_preempt,
		meicontext_noirq,
		2'h0,
		meicontext_irq,
		mie_mtie && meicontext_clearts,
		mie_msie && meicontext_clearts,
		1'b0,
		meicontext_mreteirq
	};

	default: rdata = {W_DATA{1'b0}};
	endcase
end

endmodule


`default_nettype wire

/*****************************************************************************\
|                        Copyright (C) 2022 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

`default_nettype none

// Physical memory protection unit

module hazard3_pmp #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
) (
	input  wire              clk,
	input  wire              rst_n,

	// Config interface passed through CSR block
	input  wire [11:0]       cfg_addr,
	input  wire              cfg_wen,
	input  wire [W_DATA-1:0] cfg_wdata,
	output reg  [W_DATA-1:0] cfg_rdata,

	// Fetch address query
	input  wire [W_ADDR-1:0] i_addr,
	input  wire              i_m_mode,
	output wire              i_kill,

	// Load/store address query
	input  wire [W_ADDR-1:0] d_addr,
	// Broken out separately for carry-save:
	input  wire [W_ADDR-1:0] d_addr_addend_rs1,
	input  wire [W_ADDR-1:0] d_addr_addend_imm,
	input  wire [W_ADDR-1:0] d_addr_addend_lspair_offs,
	input  wire              d_m_mode,
	input  wire              d_write,
	output wire              d_kill
);

localparam PMP_A_NAPOT = 2'b11;
localparam PMP_A_NA4   = 2'b10;
localparam PMP_A_TOR   = 2'b01;
localparam PMP_A_OFF   = 2'b00;

// Which values are supported in A field (unsupported are mapped to OFF):
localparam [3:0] PMP_A_SUPPORTED = {
	|PMP_MATCH_NAPOT,
	|PMP_MATCH_NAPOT && PMP_GRAIN == 0,
	|PMP_MATCH_TOR,
	1'b1
};

/*****************************************************************************\
|                        Copyright (C) 2022 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// List of addresses for CSRs implemented by Hazard3, including custom CSRs.

// ----------------------------------------------------------------------------
// M-mode CSRs

// Machine Information Registers (RO)
localparam MVENDORID      = 12'hf11; // Vendor ID.
localparam MARCHID        = 12'hf12; // Architecture ID.
localparam MIMPID         = 12'hf13; // Implementation ID.
localparam MHARTID        = 12'hf14; // Hardware thread ID.
localparam MCONFIGPTR     = 12'hf15; // Pointer to configuration data structure.

// Machine Trap Setup (RW)
localparam MSTATUS        = 12'h300; // Machine status register.
localparam MSTATUSH       = 12'h310; // As of priv-1.12 this must be present even if tied 0.
localparam MISA           = 12'h301; // ISA and extensions
localparam MEDELEG        = 12'h302; // Machine exception delegation register.
localparam MIDELEG        = 12'h303; // Machine interrupt delegation register.
localparam MIE            = 12'h304; // Machine interrupt-enable register.
localparam MTVEC          = 12'h305; // Machine trap-handler base address.
localparam MCOUNTEREN     = 12'h306; // Machine counter enable.

// Machine Trap Handling (RW)
localparam MSCRATCH       = 12'h340; // Scratch register for machine trap handlers.
localparam MEPC           = 12'h341; // Machine exception program counter.
localparam MCAUSE         = 12'h342; // Machine trap cause.
localparam MTVAL          = 12'h343; // Machine bad address or instruction.
localparam MIP            = 12'h344; // Machine interrupt pending.

// Machine Memory Protection (RW)
localparam PMPCFG0        = 12'h3a0; // Physical memory protection configuration.
localparam PMPCFG1        = 12'h3a1; // Physical memory protection configuration, RV32 only.
localparam PMPCFG2        = 12'h3a2; // Physical memory protection configuration.
localparam PMPCFG3        = 12'h3a3; // Physical memory protection configuration, RV32 only.
localparam PMPADDR0       = 12'h3b0; // Physical memory protection address register.
localparam PMPADDR1       = 12'h3b1; // ...
localparam PMPADDR2       = 12'h3b2;
localparam PMPADDR3       = 12'h3b3;
localparam PMPADDR4       = 12'h3b4;
localparam PMPADDR5       = 12'h3b5;
localparam PMPADDR6       = 12'h3b6;
localparam PMPADDR7       = 12'h3b7;
localparam PMPADDR8       = 12'h3b8;
localparam PMPADDR9       = 12'h3b9;
localparam PMPADDR10      = 12'h3ba;
localparam PMPADDR11      = 12'h3bb;
localparam PMPADDR12      = 12'h3bc;
localparam PMPADDR13      = 12'h3bd;
localparam PMPADDR14      = 12'h3be;
localparam PMPADDR15      = 12'h3bf;

localparam MSECCFG        = 12'h747;
localparam MSECCFGH       = 12'h757;

// Performance counters (RW)
localparam MCYCLE         = 12'hb00; // Raw cycles since start of day
localparam MINSTRET       = 12'hb02; // Instruction retire count since start of day
localparam MHPMCOUNTER3   = 12'hb03; // WARL (we tie to 0)
localparam MHPMCOUNTER4   = 12'hb04; // WARL (we tie to 0)
localparam MHPMCOUNTER5   = 12'hb05; // WARL (we tie to 0)
localparam MHPMCOUNTER6   = 12'hb06; // WARL (we tie to 0)
localparam MHPMCOUNTER7   = 12'hb07; // WARL (we tie to 0)
localparam MHPMCOUNTER8   = 12'hb08; // WARL (we tie to 0)
localparam MHPMCOUNTER9   = 12'hb09; // WARL (we tie to 0)
localparam MHPMCOUNTER10  = 12'hb0a; // WARL (we tie to 0)
localparam MHPMCOUNTER11  = 12'hb0b; // WARL (we tie to 0)
localparam MHPMCOUNTER12  = 12'hb0c; // WARL (we tie to 0)
localparam MHPMCOUNTER13  = 12'hb0d; // WARL (we tie to 0)
localparam MHPMCOUNTER14  = 12'hb0e; // WARL (we tie to 0)
localparam MHPMCOUNTER15  = 12'hb0f; // WARL (we tie to 0)
localparam MHPMCOUNTER16  = 12'hb10; // WARL (we tie to 0)
localparam MHPMCOUNTER17  = 12'hb11; // WARL (we tie to 0)
localparam MHPMCOUNTER18  = 12'hb12; // WARL (we tie to 0)
localparam MHPMCOUNTER19  = 12'hb13; // WARL (we tie to 0)
localparam MHPMCOUNTER20  = 12'hb14; // WARL (we tie to 0)
localparam MHPMCOUNTER21  = 12'hb15; // WARL (we tie to 0)
localparam MHPMCOUNTER22  = 12'hb16; // WARL (we tie to 0)
localparam MHPMCOUNTER23  = 12'hb17; // WARL (we tie to 0)
localparam MHPMCOUNTER24  = 12'hb18; // WARL (we tie to 0)
localparam MHPMCOUNTER25  = 12'hb19; // WARL (we tie to 0)
localparam MHPMCOUNTER26  = 12'hb1a; // WARL (we tie to 0)
localparam MHPMCOUNTER27  = 12'hb1b; // WARL (we tie to 0)
localparam MHPMCOUNTER28  = 12'hb1c; // WARL (we tie to 0)
localparam MHPMCOUNTER29  = 12'hb1d; // WARL (we tie to 0)
localparam MHPMCOUNTER30  = 12'hb1e; // WARL (we tie to 0)
localparam MHPMCOUNTER31  = 12'hb1f; // WARL (we tie to 0)

localparam MCYCLEH        = 12'hb80; // High halves of each counter
localparam MINSTRETH      = 12'hb82;
localparam MHPMCOUNTER3H  = 12'hb83;
localparam MHPMCOUNTER4H  = 12'hb84;
localparam MHPMCOUNTER5H  = 12'hb85;
localparam MHPMCOUNTER6H  = 12'hb86;
localparam MHPMCOUNTER7H  = 12'hb87;
localparam MHPMCOUNTER8H  = 12'hb88;
localparam MHPMCOUNTER9H  = 12'hb89;
localparam MHPMCOUNTER10H = 12'hb8a;
localparam MHPMCOUNTER11H = 12'hb8b;
localparam MHPMCOUNTER12H = 12'hb8c;
localparam MHPMCOUNTER13H = 12'hb8d;
localparam MHPMCOUNTER14H = 12'hb8e;
localparam MHPMCOUNTER15H = 12'hb8f;
localparam MHPMCOUNTER16H = 12'hb90;
localparam MHPMCOUNTER17H = 12'hb91;
localparam MHPMCOUNTER18H = 12'hb92;
localparam MHPMCOUNTER19H = 12'hb93;
localparam MHPMCOUNTER20H = 12'hb94;
localparam MHPMCOUNTER21H = 12'hb95;
localparam MHPMCOUNTER22H = 12'hb96;
localparam MHPMCOUNTER23H = 12'hb97;
localparam MHPMCOUNTER24H = 12'hb98;
localparam MHPMCOUNTER25H = 12'hb99;
localparam MHPMCOUNTER26H = 12'hb9a;
localparam MHPMCOUNTER27H = 12'hb9b;
localparam MHPMCOUNTER28H = 12'hb9c;
localparam MHPMCOUNTER29H = 12'hb9d;
localparam MHPMCOUNTER30H = 12'hb9e;
localparam MHPMCOUNTER31H = 12'hb9f;

localparam MCOUNTINHIBIT  = 12'h320; // Count inhibit register for mcycle/minstret
localparam MHPMEVENT3     = 12'h323; // WARL (we tie to 0)
localparam MHPMEVENT4     = 12'h324; // WARL (we tie to 0)
localparam MHPMEVENT5     = 12'h325; // WARL (we tie to 0)
localparam MHPMEVENT6     = 12'h326; // WARL (we tie to 0)
localparam MHPMEVENT7     = 12'h327; // WARL (we tie to 0)
localparam MHPMEVENT8     = 12'h328; // WARL (we tie to 0)
localparam MHPMEVENT9     = 12'h329; // WARL (we tie to 0)
localparam MHPMEVENT10    = 12'h32a; // WARL (we tie to 0)
localparam MHPMEVENT11    = 12'h32b; // WARL (we tie to 0)
localparam MHPMEVENT12    = 12'h32c; // WARL (we tie to 0)
localparam MHPMEVENT13    = 12'h32d; // WARL (we tie to 0)
localparam MHPMEVENT14    = 12'h32e; // WARL (we tie to 0)
localparam MHPMEVENT15    = 12'h32f; // WARL (we tie to 0)
localparam MHPMEVENT16    = 12'h330; // WARL (we tie to 0)
localparam MHPMEVENT17    = 12'h331; // WARL (we tie to 0)
localparam MHPMEVENT18    = 12'h332; // WARL (we tie to 0)
localparam MHPMEVENT19    = 12'h333; // WARL (we tie to 0)
localparam MHPMEVENT20    = 12'h334; // WARL (we tie to 0)
localparam MHPMEVENT21    = 12'h335; // WARL (we tie to 0)
localparam MHPMEVENT22    = 12'h336; // WARL (we tie to 0)
localparam MHPMEVENT23    = 12'h337; // WARL (we tie to 0)
localparam MHPMEVENT24    = 12'h338; // WARL (we tie to 0)
localparam MHPMEVENT25    = 12'h339; // WARL (we tie to 0)
localparam MHPMEVENT26    = 12'h33a; // WARL (we tie to 0)
localparam MHPMEVENT27    = 12'h33b; // WARL (we tie to 0)
localparam MHPMEVENT28    = 12'h33c; // WARL (we tie to 0)
localparam MHPMEVENT29    = 12'h33d; // WARL (we tie to 0)
localparam MHPMEVENT30    = 12'h33e; // WARL (we tie to 0)
localparam MHPMEVENT31    = 12'h33f; // WARL (we tie to 0)

// Other standard M-mode CSRs:
localparam MENVCFG        = 12'h30a;
localparam MENVCFGH       = 12'h31a;

// Custom M-mode CSRs:
localparam PMPCFGM0       = 12'hbd0; // Make PMP regions M-mode without locking
//                              bd1  // (reserved for >32 regions)

localparam MEIEA          = 12'hbe0; // External interrupt pending array
localparam MEIPA          = 12'hbe1; // External interrupt enable array
localparam MEIFA          = 12'hbe2; // External interrupt force array
localparam MEIPRA         = 12'hbe3; // External interrupt priority array
localparam MEINEXT        = 12'hbe4; // Next external interrupt
localparam MEICONTEXT     = 12'hbe5; // External interrupt context register

localparam MSLEEP         = 12'hbf0; // M-mode sleep control register
localparam H3MISA         = 12'hbf1; // Hazard3 M-mode ISA identification register

// ----------------------------------------------------------------------------
// U-mode CSRs

// Read-only aliases of M-mode counter CSRs:
localparam CYCLE          = 12'hc00;
localparam TIME           = 12'hc01;
localparam INSTRET        = 12'hc02;
localparam CYCLEH         = 12'hc80;
localparam TIMEH          = 12'hc81;
localparam INSTRETH       = 12'hc82;

// Custom U-mode CSRs
localparam SLEEP          = 12'h8f0; // U-mode subset of M-mode sleep control

// ----------------------------------------------------------------------------
// Trigger Module

localparam TSELECT       = 12'h7a0;
localparam TDATA1        = 12'h7a1;
localparam TDATA2        = 12'h7a2;
localparam TDATA3        = 12'h7a3;
localparam TINFO         = 12'h7a4;
localparam TCONTROL      = 12'h7a5;
localparam MCONTEXT      = 12'h7a8;

// ----------------------------------------------------------------------------
// D-mode CSRs

localparam DCSR           = 12'h7b0;
localparam DPC            = 12'h7b1;
localparam DMDATA0        = 12'hbff; // Custom read/write

generate
if (PMP_REGIONS == 0) begin: no_pmp

// This should already be stubbed out in core.v, but use a generate here too
// so that we don't get a warning for elaborating this module with a region
// count of 0.

always @ (*) cfg_rdata = {W_DATA{1'b0}};
assign i_kill = 1'b0;
assign d_kill = 1'b0;

end else begin: have_pmp

// ----------------------------------------------------------------------------
// Config registers and read/write interface

// Whether a region's configuration is writable; this is non-trivial when TOR
// is supported because locking region i + 1 can also lock region i.
wire [PMP_REGIONS-1:0] region_locked;

reg  [PMP_REGIONS-1:0] pmpcfg_l;
reg  [1:0]             pmpcfg_a [0:PMP_REGIONS-1];
reg  [PMP_REGIONS-1:0] pmpcfg_x;
reg  [PMP_REGIONS-1:0] pmpcfg_w;
reg  [PMP_REGIONS-1:0] pmpcfg_r;

// Address register contains bits 33:2 of the address (to support 16 GiB
// physical address space). We don't implement bits 33 or 32.
reg  [W_ADDR-3:0]      pmpaddr  [0:PMP_REGIONS-1];

// Hazard3 extension for applying PMP regions to M-mode without locking.
// Different from ePMP mseccfg.rlb: low-numbered regions may be locked for
// security reasons, but higher-numbered regions should stll be available for
// other purposes e.g. stack guarding, peripheral emulation
reg  [PMP_REGIONS-1:0] pmpcfg_m;

always @ (posedge clk or negedge rst_n) begin: cfg_update
	reg signed [31:0] i;
	if (!rst_n) begin
		for (i = 0; i < PMP_REGIONS; i = i + 1) begin
			pmpcfg_l[i] <= PMP_HARDWIRED[i] ? PMP_HARDWIRED_CFG[8 * i + 7]      : 1'b0;
			pmpcfg_a[i] <= PMP_HARDWIRED[i] ? PMP_HARDWIRED_CFG[8 * i + 3 +: 2] : 2'h0;
			pmpcfg_x[i] <= PMP_HARDWIRED[i] ? PMP_HARDWIRED_CFG[8 * i + 2]      : 1'b0;
			pmpcfg_w[i] <= PMP_HARDWIRED[i] ? PMP_HARDWIRED_CFG[8 * i + 1]      : 1'b0;
			pmpcfg_r[i] <= PMP_HARDWIRED[i] ? PMP_HARDWIRED_CFG[8 * i + 0]      : 1'b0;

			pmpaddr[i]  <= PMP_HARDWIRED[i] ? PMP_HARDWIRED_ADDR[32 * i +: 30]  :
			               PMP_GRAIN > 1    ? ~(~30'h0 << (PMP_GRAIN - 1))      : 30'h0;
		end
		pmpcfg_m <= {PMP_REGIONS{1'b0}};
	end else if (cfg_wen) begin
		for (i = 0; i < PMP_REGIONS; i = i + 1) begin
			if (cfg_addr == PMPCFG0 + i[13:2] && !region_locked[i]) begin
				if (PMP_HARDWIRED[i]) begin
					// Keep tied to hardwired value (but still make the "register" sensitive to clk)
					pmpcfg_l[i] <= PMP_HARDWIRED_CFG[8 * i + 7];
					pmpcfg_a[i] <= PMP_HARDWIRED_CFG[8 * i + 3 +: 2];
					pmpcfg_x[i] <= PMP_HARDWIRED_CFG[8 * i + 2];
					pmpcfg_w[i] <= PMP_HARDWIRED_CFG[8 * i + 1];
					pmpcfg_r[i] <= PMP_HARDWIRED_CFG[8 * i + 0];
					pmpaddr[i]  <= PMP_HARDWIRED_ADDR[32 * i +: 30];
				end else begin
					pmpcfg_l[i] <= cfg_wdata[i % 4 * 8 + 7];
					pmpcfg_x[i] <= cfg_wdata[i % 4 * 8 + 2];
					pmpcfg_w[i] <= cfg_wdata[i % 4 * 8 + 1];
					pmpcfg_r[i] <= cfg_wdata[i % 4 * 8 + 0];
					// Unsupported A values are mapped to OFF (it's a WARL field).
					pmpcfg_a[i] <= PMP_A_SUPPORTED[cfg_wdata[i % 4 * 8 + 3 +: 2]] ?
						cfg_wdata[i % 4 * 8 + 3 +: 2] : PMP_A_OFF;
				end
			end
			if (cfg_addr == PMPADDR0 + i[11:0] && !region_locked[i]) begin
				// This implements one bit too many when G > 0 and only
				// PMP_MATCH_TOR is enabled, however that bit is ignored for
				// both rdata and address matching, so should be trimmed.
				if (PMP_GRAIN > 1) begin
					pmpaddr[i] <= cfg_wdata[W_ADDR-3:0] | ~(~30'h0 << (PMP_GRAIN - 1));
				end else begin
					pmpaddr[i] <= cfg_wdata[W_ADDR-3:0];
				end
			end
		end
		if (cfg_addr == PMPCFGM0) begin
			pmpcfg_m <= cfg_wdata[PMP_REGIONS-1:0] & ~PMP_HARDWIRED & {PMP_REGIONS{|EXTENSION_XH3PMPM}};
		end
	end
end


always @ (*) begin: cfg_read
	reg signed [31:0] i;
	cfg_rdata = {W_DATA{1'b0}};
	for (i = 0; i < PMP_REGIONS; i = i + 1) begin
		if (cfg_addr == PMPCFG0 + i[13:2]) begin
			cfg_rdata[i % 4 * 8 +: 8] = {
				pmpcfg_l[i],
				2'b00,
				pmpcfg_a[i],
				pmpcfg_x[i],
				pmpcfg_w[i],
				pmpcfg_r[i]
			};
		end else if (cfg_addr == PMPADDR0 + i[11:0]) begin
			if (PMP_GRAIN >= 2 && pmpcfg_a[i][1]) begin
				// Bits G-2:0 read back as all-ones when A is NA4 or NAPOT.
				cfg_rdata[W_ADDR-3:0] = pmpaddr[i] | ~({W_ADDR-2{1'b1}} << (PMP_GRAIN - 1));
			end else if (PMP_GRAIN >= 1 && !pmpcfg_a[i][1]) begin
				// Bits G-1:0 read back as all-zeroes when A is OFF or TOR.
				cfg_rdata[W_ADDR-3:0] = pmpaddr[i] & ({W_ADDR-2{1'b1}} << PMP_GRAIN);
			end else begin
				cfg_rdata[W_ADDR-3:0] = pmpaddr[i];
			end
		end
	end
	if (cfg_addr == PMPCFGM0) begin
		cfg_rdata = {{32-PMP_REGIONS{1'b0}}, pmpcfg_m} & {32{|EXTENSION_XH3PMPM}};
	end
end

// ----------------------------------------------------------------------------
// Region locking rules

reg [PMP_REGIONS-1:0] pmp_region_is_tor;
always @ (*) begin: check_region_is_tor
	integer i;
	for (i = 0; i < PMP_REGIONS; i = i + 1) begin
		pmp_region_is_tor[i] = PMP_MATCH_TOR && pmpcfg_a[i] == PMP_A_TOR;
	end
end

assign region_locked = pmpcfg_l | ((pmpcfg_l & pmp_region_is_tor) >> 1);

// ----------------------------------------------------------------------------
// Match addresses against regions

wire [PMP_REGIONS-1:0] d_match_napot;
wire [PMP_REGIONS-1:0] i_match_napot;
wire [PMP_REGIONS-1:0] d_match_tor;
wire [PMP_REGIONS-1:0] i_match_tor;

if (PMP_MATCH_NAPOT != 0) begin: have_napot

	reg [PMP_REGIONS-1:0] d_match_napot_r;
	reg [PMP_REGIONS-1:0] i_match_napot_r;

	assign d_match_napot = d_match_napot_r;
	assign i_match_napot = i_match_napot_r;

	// Decode PMPCFGx.A and PMPADDRx into a 32-bit address mask and address
	reg [W_ADDR-1:0] match_mask [0:PMP_REGIONS-1];
	reg [W_ADDR-1:0] match_addr [0:PMP_REGIONS-1];

	// Encoding: (noting ADDR is a 4-byte address, not a word address):
	// CFG.A |  ADDR    | Region size
	// ------+----------+------------
	// NA4   | y..yyyyy | 4 bytes
	// NAPOT | y..yyyy0 | 8 bytes
	// NAPOT | y..yyy01 | 16 bytes
	// NAPOT | y..yy011 | 32 bytes
	// NAPOT | y..y0111 | 64 bytes
	// etc.
	//
	// So, with the exception of NA4, the rule is to check all bits more
	// significant than the least-significant 0 bit.

	always @ (*) begin: decode_match_mask_addr
		integer i, j;
		for (i = 0; i < PMP_REGIONS; i = i + 1) begin
			if (!pmpcfg_a[i][0]) begin
				match_mask[i] = {{W_ADDR-2{1'b1}}, 2'b00};
			end else begin
				// Bits 1:0 are always 0. Bit 2 is 0 because NAPOT is at least 8 bytes.
				match_mask[i] = {W_ADDR{1'b0}};
				for (j = 3; j < W_ADDR; j = j + 1) begin
					match_mask[i][j] = match_mask[i][j - 1] || !pmpaddr[i][j - 3];
				end
			end
			match_addr[i] = {pmpaddr[i], 2'b00} & match_mask[i];
		end
	end

	// We check only the least-addressed byte of each access. See later
	// comments for an argument as to why this is sufficient.

	always @ (*) begin: check_d_match
		integer i;
		for (i = PMP_REGIONS - 1; i >= 0; i = i - 1) begin
			d_match_napot_r[i] = pmpcfg_a[i][1] &&
				(d_addr & match_mask[i]) == match_addr[i];
			i_match_napot_r[i] = pmpcfg_a[i][1] &&
				(i_addr & match_mask[i]) == match_addr[i];
		end
	end

end else begin: no_napot

	assign d_match_napot = {PMP_REGIONS{1'b0}};
	assign i_match_napot = {PMP_REGIONS{1'b0}};

end

if (PMP_MATCH_TOR != 0) begin: have_tor

	reg [PMP_REGIONS-1:0] d_match_tor_r;
	reg [PMP_REGIONS-1:0] i_match_tor_r;
	reg [W_ADDR-1:0]      watermark [0:PMP_REGIONS-1];
	reg [PMP_REGIONS-1:0] d_lt;
	reg [PMP_REGIONS-1:0] i_lt;

	assign d_match_tor = d_match_tor_r;
	assign i_match_tor = i_match_tor_r;

	always @ (*) begin: compare
		integer i;
		for (i = 0; i < PMP_REGIONS; i = i + 1) begin
			watermark[i] = {
				pmpaddr[i][W_ADDR-3:0] & (~30'h0 << PMP_GRAIN),
				2'b00
			};
			// Bring terms in separately to try to encourage adder merging
			d_lt[i] = (
				d_addr_addend_rs1 + d_addr_addend_imm + d_addr_addend_lspair_offs
			) < watermark[i];
			i_lt[i] = i_addr < watermark[i];
		end
	end

	wire [PMP_REGIONS-1:0] d_prev_ge = ~(d_lt << 1);
	wire [PMP_REGIONS-1:0] i_prev_ge = ~(i_lt << 1);

	always @ (*) begin: match
		integer i;
		for (i = 0; i < PMP_REGIONS; i = i + 1) begin
			d_match_tor_r[i] = d_lt[i] && d_prev_ge[i] && pmpcfg_a[i] == PMP_A_TOR;
			i_match_tor_r[i] = i_lt[i] && i_prev_ge[i] && pmpcfg_a[i] == PMP_A_TOR;
		end
	end

end else begin: no_tor

	assign d_match_tor = {PMP_REGIONS{1'b0}};
	assign i_match_tor = {PMP_REGIONS{1'b0}};

end

// ----------------------------------------------------------------------------
// Decode permissions from matches

// For load/stores we assume any non-naturally-aligned transfers trigger a
// misaligned load/store/AMO exception, so we only need to decode the PMP
// attribute for the first byte of the access. Note the spec gives us freedom
// to report *either* a load/store/AMO access fault (mcause = 5, 7) or a
// load/store/AMO alignment fault (mcause = 4, 6), in the case that both
// happen, and we choose alignment fault in this case.

reg d_m; // Hazard3 extension (M-mode without locking)
reg d_l;
reg d_r;
reg d_w;

always @ (*) begin: check_d_match
	integer i;
	d_m = 1'b0;
	d_l = 1'b0;
	d_r = 1'b0;
	d_w = 1'b0;
	// Lowest-numbered match wins, so work down from the top. This should be
	// inferred as a priority mux structure (cascade mux).
	for (i = PMP_REGIONS - 1; i >= 0; i = i - 1) begin
		if (d_match_napot[i] || d_match_tor[i]) begin
			d_m = pmpcfg_m[i];
			d_l = pmpcfg_l[i];
			d_r = pmpcfg_r[i];
			d_w = pmpcfg_w[i];
		end
	end
end

// Instructions work similarly because we check *fetches*, not instructions.
// Fetch is always word-sized word-aligned. The spec permits this:
//
// "On some implementations, misaligned loads, stores, and instruction fetches
//  may also be decomposed into multiple accesses, some of which may succeed
//  before an access-fault exception occurs."
//
// Hazard3 separately checks the naturally-aligned fetches that occur in the
// course of fetching a non-naturally-aligned instruction. This means
// instruction fetch spanning two different regions which both grant X
// permission *is* permitted, unlike the RP2350 version of Hazard3.

reg i_m; // Hazard3 extension (M-mode without locking)
reg i_l;
reg i_x;

always @ (*) begin: check_i_match
	integer i;
	i_m = 1'b0;
	i_l = 1'b0;
	i_x = 1'b0;
	for (i = PMP_REGIONS - 1; i >= 0; i = i - 1) begin
		if (i_match_napot[i] || i_match_tor[i]) begin
			i_m = pmpcfg_m[i];
			i_l = pmpcfg_l[i];
			i_x = pmpcfg_x[i];
		end
	end
end

// ----------------------------------------------------------------------------
// Access rules

// M-mode gets to ignore protections, unless the lock or M-mode bit is set.

assign d_kill = (!d_m_mode || d_l || d_m) && (
	(!d_write && !d_r) ||
	( d_write && !d_w)
);

assign i_kill =	(!i_m_mode || i_l || i_m) && !i_x;

end
endgenerate

endmodule


`default_nettype wire

/*****************************************************************************\
|                        Copyright (C) 2022 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

`default_nettype none

// Wake/sleep (power) state machine for Hazard3

module hazard3_power_ctrl #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
) (
	input  wire              clk_always_on,
	input  wire              rst_n,

	// 4-phase (Gray code) req/ack handshake for requesting and releasing
	// power+clock enable on non-processor hardware, e.g. the bus fabric. This
	// can also be used for an external controller to gate the processor's clk
	// input, rather than the clk_en signal below.
	output reg               pwrup_req,
	input  wire              pwrup_ack,

	// Top-level clock enable for an optional clock gate on the processor's clk
	// input (but not clk_always_on, which clocks this module and the IRQ input
	// flops). This allows the processor to clock-gate when sleeping. It's
	// acceptable for the clock gate cell to have one cycle of delay when
	// clk_en changes.
	output reg               clk_en,

	// Power state controls from CSRs
	input  wire              allow_clkgate,
	input  wire              allow_power_down,
	input  wire              allow_sleep_on_block,

	// Signal from frontend that it has stalled against the WFI pipeline
	// stall, and we are now clear to enter a deep sleep state
	input  wire              frontend_pwrdown_ok,

	input  wire              sleeping_on_wfi,
	input  wire              wfi_wakeup_req,
	input  wire              sleeping_on_block,
	input  wire              block_wakeup_req_pulse,
	output reg               stall_release
);

// ----------------------------------------------------------------------------
// Wake/sleep state machine

localparam W_STATE              = 2;
localparam S_AWAKE              = 2'h0;
localparam S_ENTER_ASLEEP       = 2'h1;
localparam S_ASLEEP             = 2'h2;
localparam S_ENTER_AWAKE        = 2'h3;

reg [W_STATE-1:0] state;
reg               block_wakeup_req;

wire active_wake_req =
	(sleeping_on_block && (block_wakeup_req || wfi_wakeup_req)) ||
	(sleeping_on_wfi && wfi_wakeup_req);

// Note: we assert our power up request during reset, and *assume* that the
// power up acknowledge is also high at reset. If this is a problem, extend
// the core reset.

always @ (posedge clk_always_on or negedge rst_n) begin
	if (!rst_n) begin
		state <= S_AWAKE;
		pwrup_req <= 1'b1;
		clk_en <= 1'b1;
		stall_release <= 1'b0;
	end else begin
		stall_release <= 1'b0;
		case (state)
		S_AWAKE: if (sleeping_on_wfi || sleeping_on_block) begin
			if (stall_release) begin
				// The last cycle of an ongoing which we have just released. Sit
				// tight, this instruction will move down the pipeline at the
				// end of this cycle. (There is an assertion that this doesn't
				// happen twice.)
				state <= S_AWAKE;
			end else if (active_wake_req) begin
				// Skip deep sleep if it would immediately fall through.
				stall_release <= 1'b1;
			end else if ((allow_power_down || allow_clkgate) && (sleeping_on_wfi || allow_sleep_on_block)) begin
				if (frontend_pwrdown_ok) begin
					pwrup_req <= !allow_power_down;
					clk_en <= !allow_clkgate;
					state <= allow_power_down ? S_ENTER_ASLEEP : S_ASLEEP;
				end else begin
					// Stay awake until it is safe to power down (i.e. until our
					// instruction fetch goes quiet).
					state <= S_AWAKE;
				end					
			end else begin
				// No power state change. Just sit with the pipeline stalled.
				state <= S_AWAKE;
			end
		end
		S_ENTER_ASLEEP: if (!pwrup_ack) begin
			state <= S_ASLEEP;
		end
		S_ASLEEP: if (active_wake_req) begin
			pwrup_req <= 1'b1;
			clk_en <= 1'b1;
			// Still go through the enter state for non-power-down wakeup, in
			// case the clock gate cell has a 1 cycle delay.
			state <= S_ENTER_AWAKE;
		end
		S_ENTER_AWAKE: if (pwrup_ack || !allow_power_down) begin
			state <= S_AWAKE;
			stall_release <= 1'b1;
		end
		default: begin
			state <= S_AWAKE;
		end
		endcase
	end
end































// ----------------------------------------------------------------------------
// Pulse->level for block wakeup

// Unblock signal is sticky: a prior unblock with no block since will cause
// the next block to immediately fall through.

always @ (posedge clk_always_on or negedge rst_n) begin
	if (!rst_n) begin
		block_wakeup_req <= 1'b0;
	end else begin
		// Note the OR takes precedence over the AND, so we don't miss a second
		// unblock that arrives at the instant we wake up.
		block_wakeup_req <= (block_wakeup_req && !(
			sleeping_on_block && stall_release
		)) || block_wakeup_req_pulse;
	end
end

endmodule


`default_nettype wire

/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Register file
// Single write port, dual read port

`default_nettype none

module hazard3_regfile_1w2r #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
) (
	input wire clk,
	input wire rst_n,

	input wire [4:0]        raddr1,
	output reg [W_DATA-1:0] rdata1,

	input wire [4:0]        raddr2,
	output reg [W_DATA-1:0] rdata2,

	input wire [4:0]        waddr,
	input wire [W_DATA-1:0] wdata,
	input wire              wen
);

localparam       N_REGS      = EXTENSION_E == 0 ? 32 : 16;
localparam [4:0] REGNUM_MASK = {~|EXTENSION_E, 4'hf};

wire [4:0] raddr1_masked = raddr1 & REGNUM_MASK;
wire [4:0] raddr2_masked = raddr2 & REGNUM_MASK;
wire [4:0] waddr_masked  = waddr  & REGNUM_MASK;

generate
if (RESET_REGFILE) begin: real_dualport_reset
	// This will presumably always be implemented with flops
	reg [W_DATA-1:0] mem [0:N_REGS-1];

	integer i;
	always @ (posedge clk or negedge rst_n) begin
		if (!rst_n) begin
			for (i = 0; i < N_REGS; i = i + 1) begin
				mem[i] <= {W_DATA{1'b0}};
			end
			rdata1 <= {W_DATA{1'b0}};
			rdata2 <= {W_DATA{1'b0}};
		end else begin
			if (wen) begin
				mem[waddr_masked] <= wdata;
			end
			rdata1 <= mem[raddr1_masked];
			rdata2 <= mem[raddr2_masked];
		end
	end
end else begin: real_dualport_noreset
	// This should be inference-compatible on FPGAs with dual-port (or 1R1W) BRAMs
	





	// Optionally force use of distributed RAM on Xilinx for better timing
	


	reg [W_DATA-1:0] mem [0:N_REGS-1];

	always @ (posedge clk) begin
		if (wen) begin
			mem[waddr_masked] <= wdata;
		end
		rdata1 <= mem[raddr1_masked];
		rdata2 <= mem[raddr2_masked];
	end
end
endgenerate

endmodule


`default_nettype wire

/*****************************************************************************\
|                        Copyright (C) 2022 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

`default_nettype none

// The Hazard3 trigger unit always implements one trigger of each of the
// following types:
//
// * Instruction count trigger (type=3) with count=1 (can single-step U-mode
//   from M-mode, or step M-mode foreground from an M-mode exception handler)
//
// * Interrupt trigger (type=4): trigger on mask of mtip/msip/meip interrupts
//
// * Exception trigger (type=5): trigger on mask of exception causes
//
// The following are optionally supported:
//
// * Instruction address triggers (type=2 execute=1 select=0) aka breakpoints
//
// Breakpoints always use exact address matches, and the timing is always
// "early". The number of breakpoints is configured by BREAKPOINT_TRIGGERS,
// which can be 0.
//
// Interrupt/exception triggers break after the core transfers to its trap
// handler, but before the first trap handler instruction executes. Only
// action=1 is supported for these triggers, since trap-on-trap is useless
// when M is the only privileged mode.

module hazard3_triggers #(
/*****************************************************************************\
|                      Copyright (C) 2021-2022 Luke Wren                      |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// Hazard3 CPU configuration parameters

// To configure Hazard3 you can either edit this file, or set parameters on
// your top-level instantiation, it's up to you. These parameters are all
// plumbed through Hazard3's internal hierarchy to the appropriate places.

// If you add a parameter here, you should add a matching line to
// hazard3_config_inst.vh to propagate the parameter through module
// instantiations.

// ----------------------------------------------------------------------------
// Reset state configuration

// RESET_VECTOR: Address of first instruction executed.
parameter RESET_VECTOR        = 32'h00000000,

// MTVEC_INIT: Initial value of trap vector base. Bits clear in MTVEC_WMASK
// will never change from this initial value. Bits set in MTVEC_WMASK can be
// written/set/cleared as normal.
//
// Note that mtvec bits 1:0 do not affect the trap base (as per RISC-V spec).
// Bit 1 is don't care, bit 0 selects the vectoring mode: unvectored if == 0
// (all traps go to mtvec), vectored if == 1 (exceptions go to mtvec, IRQs to
// mtvec + mcause * 4). This means MTVEC_INIT also sets the initial vectoring
// mode.
parameter MTVEC_INIT          = 32'h00000000,

// ----------------------------------------------------------------------------
// Standard RISC-V ISA support

// EXTENSION_A: Support for atomic read/modify/write instructions
parameter EXTENSION_A         = 1,

// EXTENSION_C: Support for compressed (variable-width) instructions
parameter EXTENSION_C         = 1,

// EXTENSION_E: Implement the RV32E base extension rather than RV32I. This
// reduces the number of integer registers from 31 to 15.
parameter EXTENSION_E         = 0,

// EXTENSION_M: Support for hardware multiply/divide/modulo instructions
parameter EXTENSION_M         = 1,

// EXTENSION_ZBA: Support for Zba address generation instructions
parameter EXTENSION_ZBA       = 0,

// EXTENSION_ZBB: Support for Zbb basic bit manipulation instructions
parameter EXTENSION_ZBB       = 0,

// EXTENSION_ZBC: Support for Zbc carry-less multiplication instructions
parameter EXTENSION_ZBC       = 0,

// EXTENSION_ZBKB: Support for Zbkb basic bit manipulation for cryptography
// Requires: Zbb. (This flag enables instructions in Zbkb which aren't in Zbb.)
parameter EXTENSION_ZBKB      = 0,

// EXTENSION_ZBKX: support for Zbkx crossbar permutation instructions
parameter EXTENSION_ZBKX      = 0,

// EXTENSION_ZBS: Support for Zbs single-bit manipulation instructions
parameter EXTENSION_ZBS       = 0,

// EXTENSION_ZCB: Support for Zcb basic additional compressed instructions
// Requires: EXTENSION_C. (Some Zcb instructions also require Zbb or M.)
// Note Zca is equivalent to C, as we do not support the F extension.
parameter EXTENSION_ZCB       = 0,

// EXTENSION_ZCLSD: Support for Zclsd compressed load/store pair instructions
// Requires: EXTENSION_ZILSD, EXTENSION_C.
parameter EXTENSION_ZCLSD     = 0,

// EXTENSION_ZCMP: Support for Zcmp push/pop instructions.
// Requires: EXTENSION_C.
parameter EXTENSION_ZCMP      = 0,

// EXTENSION_ZIFENCEI: Support for the fence.i instruction
// Optional, since a plain branch/jump will also flush the prefetch queue.
parameter EXTENSION_ZIFENCEI  = 0,

// EXTENSION_ZILSD: Support for Zilsd load/store pair instructions
parameter EXTENSION_ZILSD     = 0,

// ----------------------------------------------------------------------------
// Custom RISC-V extensions

// EXTENSION_XH3B: Custom bit-extract-multiple instructions for Hazard3
parameter EXTENSION_XH3BEXTM  = 0,

// EXTENSION_XH3IRQ: Custom preemptive, prioritised interrupt support. Can be
// disabled if an external interrupt controller (e.g. PLIC) is used. If
// disabled, and NUM_IRQS > 1, the external interrupts are simply OR'd into
// mip.meip.
parameter EXTENSION_XH3IRQ    = 0,

// EXTENSION_XH3PMPM: PMPCFGMx CSRs to enforce PMP regions in M-mode without
// locking. Unlike ePMP mseccfg.rlb, locked and unlocked regions can coexist
parameter EXTENSION_XH3PMPM   = 0,

// EXTENSION_XH3POWER: Custom power management controls for Hazard3
parameter EXTENSION_XH3POWER  = 0,

// ----------------------------------------------------------------------------
// Standard CSR support

// Note the Zicsr extension is implied by any of CSR_M_MANDATORY, CSR_M_TRAP,
// CSR_COUNTER.

// CSR_M_MANDATORY: Bare minimum CSR support e.g. misa. Spec says must = 1 if
// CSRs are present, but I won't tell anyone.
parameter CSR_M_MANDATORY     = 1,

// CSR_M_TRAP: Include M-mode trap-handling CSRs, and enable trap support.
parameter CSR_M_TRAP          = 1,

// CSR_COUNTER: Include performance counters and Zicntr CSRs
parameter CSR_COUNTER         = 0,

// U_MODE: Support the U (user) execution mode. In U mode, the core performs
// unprivileged bus accesses, and software's access to CSRs is restricted.
// Additionally, if the PMP is included, the core may restrict U-mode
// software's access to memory.
// Requires: CSR_M_TRAP.
parameter U_MODE              = 0,

// PMP_REGIONS: Number of physical memory protection regions, or 0 for no PMP.
// PMP is more useful if U mode is supported, but this is not a requirement.
parameter PMP_REGIONS         = 0,

// PMP_GRAIN: This is the "G" parameter in the privileged spec. Minimum PMP
// region size is 1 << (G + 2) bytes.  If G > 0, PMCFG.A can not be set to
// NA4 (will get set to OFF instead). If G > 1, the G - 1 LSBs of pmpaddr are
// read-only-0 when PMPCFG.A is OFF, and read-only-1 when PMPCFG.A is NAPOT.
parameter PMP_GRAIN           = 0,

// PMP_MATCH_NAPOT: Enable PMP support for the NAPOT (naturally-aligned
// power-of-two) and NA4 (naturally-aligned four-byte) matching modes. When
// disabled, attempting to select these modes will set the PMP region to OFF.
parameter PMP_MATCH_NAPOT     = 1,

// PMP_MATCH_TOR: Enable PMP support for the TOR (top-of-range) matching mode.
// When disabled, attempting to select this mode will set the region to OFF.
parameter PMP_MATCH_TOR       = 0,

// PMPADDR_HARDWIRED: If a bit is 1, the corresponding region's pmpaddr and
// pmpcfg registers are read-only. PMP_GRAIN is ignored on hardwired regions.
// It's recommended to make hardwired regions the highest-numbered, so they
// can be overridden by lower-numbered regions.
parameter PMP_HARDWIRED       = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){1'b0}},

// PMPADDR_HARDWIRED_ADDR: Values of pmpaddr registers whose PMP_HARDWIRED
// bits are set to 1. Non-hardwired regions reset to all-zeroes.
parameter PMP_HARDWIRED_ADDR  = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){32'h0}},

// PMPCFG_RESET_VAL: Values of pmpcfg registers whose PMP_HARDWIRED bits are
// set to 1. Non-hardwired regions reset to all zeroes.
parameter PMP_HARDWIRED_CFG   = {(PMP_REGIONS > 0 ? PMP_REGIONS : 1){8'h00}},

// DEBUG_SUPPORT: Support for run/halt and instruction injection from an
// external Debug Module, support for Debug Mode, and Debug Mode CSRs.
// Requires: CSR_M_MANDATORY, CSR_M_TRAP.
parameter DEBUG_SUPPORT       = 0,

// BREAKPOINT_TRIGGERS: Number of triggers which support type=2 execute=1
// (but not store/load=1, i.e. not a watchpoint). Requires: DEBUG_SUPPORT
parameter BREAKPOINT_TRIGGERS = 0,

// ----------------------------------------------------------------------------
// External interrupt support

// NUM_IRQS: Number of external IRQs. Minimum 1, maximum 512. Note that if
// EXTENSION_XH3IRQ (Hazard3 interrupt controller) is disabled then multiple
// external interrupts are simply OR'd into mip.meip.
parameter NUM_IRQS            = 1,

// IRQ_PRIORITY_BITS: Number of priority bits implemented for each interrupt
// in meipra, if EXTENSION_XH3IRQ is enabled. The number of distinct levels
// is (1 << IRQ_PRIORITY_BITS). Minimum 0, max 4. Note that multiple priority
// levels with a large number of IRQs will have a severe effect on timing.
parameter IRQ_PRIORITY_BITS   = 0,

// IRQ_INPUT_BYPASS: disable the input registers on the external interrupts,
// to reduce latency by one cycle. Can be applied on an IRQ-by-IRQ basis.
// Ignored if EXTENSION_XH3IRQ is disabled.
parameter IRQ_INPUT_BYPASS    = {(NUM_IRQS > 0 ? NUM_IRQS : 1){1'b0}},

// ----------------------------------------------------------------------------
// ID registers

// JEDEC JEP106-compliant vendor ID, can be left at 0 if "not implemented or
// [...] this is a non-commercial implementation" (RISC-V spec).
// 31:7 is continuation code count, 6:0 is ID. Parity bit is not stored.
parameter MVENDORID_VAL       = 32'h0,

// Pointer to configuration structure blob, or all-zeroes. Must be at least
// 4-byte-aligned.
parameter MCONFIGPTR_VAL      = 32'h0,

// ----------------------------------------------------------------------------
// Performance/size options

// REDUCED_BYPASS: Remove all forwarding paths except X->X (so back-to-back
// ALU ops can still run at 1 CPI), to save area.
parameter REDUCED_BYPASS      = 0,

// MULDIV_UNROLL: Bits per clock for multiply/divide circuit, if present. Must
// be a power of 2.
parameter MULDIV_UNROLL       = 1,

// MUL_FAST: Use single-cycle multiply circuit for MUL instructions, retiring
// to stage 3. The sequential multiply/divide circuit is still used for MULH*
parameter MUL_FAST            = 0,

// MUL_FASTER: Retire fast multiply results to stage 2 instead of stage 3.
// Throughput is the same, but latency is reduced from 2 cycles to 1 cycle.
// Requires: MUL_FAST.
parameter MUL_FASTER          = 0,

// MULH_FAST: extend the fast multiply circuit to also cover MULH*, and remove
// the multiply functionality from the sequential multiply/divide circuit.
// Requires: MUL_FAST
parameter MULH_FAST           = 0,

// FAST_BRANCHCMP: Instantiate a separate comparator (eq/lt/ltu) for branch
// comparisons, rather than using the ALU. Improves fetch address delay,
// especially if Zba extension is enabled. Disabling may save area.
parameter FAST_BRANCHCMP      = 1,

// RESET_REGFILE: whether to support reset of the general purpose registers.
// There are around 1k bits in the register file, so the reset can be
// disabled e.g. to permit block-RAM inference on FPGA.
parameter RESET_REGFILE       = 0,

// BRANCH_PREDICTOR: enable branch prediction. The branch predictor consists
// of a single BTB entry which is allocated on a taken backward branch, and
// cleared on a mispredicted nontaken branch, a fence.i or a trap. Successful
// prediction eliminates the 1-cyle fetch bubble on a taken branch, usually
// making tight loops faster.
parameter BRANCH_PREDICTOR    = 0,

// MTVEC_WMASK: Mask of which bits in mtvec are writable. Full writability is
// recommended, because a common idiom in setup code is to set mtvec just
// past code that may trap, as a hardware "try {...} catch" block.
//
// - The vectoring mode can be made fixed by clearing the LSB of MTVEC_WMASK
//
// - In vectored mode, the vector table must be aligned to its size, rounded
//   up to a power of two.
parameter MTVEC_WMASK         = 32'hfffffffd,

// ----------------------------------------------------------------------------
// Port size parameters (do not modify)

parameter W_ADDR              = 32,   // Do not modify
parameter W_DATA              = 32    // Do not modify
) (
	input  wire              clk,
	input  wire              rst_n,

	// Config interface passed through CSR block
	input  wire [11:0]       cfg_addr,
	input  wire              cfg_wen,
	input  wire [W_DATA-1:0] cfg_wdata,
	output reg  [W_DATA-1:0] cfg_rdata,

	// Global trigger-to-M-mode enable from tcontrol
	input  wire              trig_m_en,

	// Fetch address query from stage F
	input  wire [W_ADDR-1:0] fetch_addr,
	input  wire              fetch_m_mode,
	input  wire              fetch_d_mode,

	// Trap trigger events from stage X
	input  wire              event_instr_ret,

	// Trap trigger events from stage M
	input  wire              event_interrupt,
	input  wire              event_exception,
	input  wire [3:0]        event_trap_cause,
	input  wire              event_trap_enter,

	// F-aligned break request (for each halfword of word-sized word-aligned fetch)
	output wire [1:0]        break_any,
	output wire [1:0]        break_d_mode,

	// X-aligned step break request (to M-mode only)
	output wire              break_m_step,

	// Stage-X debug mode flag, for CSR protection (may or may not be the same
	// as the query debug mode flag)
	input  wire              x_d_mode,
	// Stage-X M-mode flag, for enables on interrupt/exception triggers
	input  wire              x_m_mode
);

/*****************************************************************************\
|                        Copyright (C) 2022 Luke Wren                         |
|                     SPDX-License-Identifier: Apache-2.0                     |
\*****************************************************************************/

// List of addresses for CSRs implemented by Hazard3, including custom CSRs.

// ----------------------------------------------------------------------------
// M-mode CSRs

// Machine Information Registers (RO)
localparam MVENDORID      = 12'hf11; // Vendor ID.
localparam MARCHID        = 12'hf12; // Architecture ID.
localparam MIMPID         = 12'hf13; // Implementation ID.
localparam MHARTID        = 12'hf14; // Hardware thread ID.
localparam MCONFIGPTR     = 12'hf15; // Pointer to configuration data structure.

// Machine Trap Setup (RW)
localparam MSTATUS        = 12'h300; // Machine status register.
localparam MSTATUSH       = 12'h310; // As of priv-1.12 this must be present even if tied 0.
localparam MISA           = 12'h301; // ISA and extensions
localparam MEDELEG        = 12'h302; // Machine exception delegation register.
localparam MIDELEG        = 12'h303; // Machine interrupt delegation register.
localparam MIE            = 12'h304; // Machine interrupt-enable register.
localparam MTVEC          = 12'h305; // Machine trap-handler base address.
localparam MCOUNTEREN     = 12'h306; // Machine counter enable.

// Machine Trap Handling (RW)
localparam MSCRATCH       = 12'h340; // Scratch register for machine trap handlers.
localparam MEPC           = 12'h341; // Machine exception program counter.
localparam MCAUSE         = 12'h342; // Machine trap cause.
localparam MTVAL          = 12'h343; // Machine bad address or instruction.
localparam MIP            = 12'h344; // Machine interrupt pending.

// Machine Memory Protection (RW)
localparam PMPCFG0        = 12'h3a0; // Physical memory protection configuration.
localparam PMPCFG1        = 12'h3a1; // Physical memory protection configuration, RV32 only.
localparam PMPCFG2        = 12'h3a2; // Physical memory protection configuration.
localparam PMPCFG3        = 12'h3a3; // Physical memory protection configuration, RV32 only.
localparam PMPADDR0       = 12'h3b0; // Physical memory protection address register.
localparam PMPADDR1       = 12'h3b1; // ...
localparam PMPADDR2       = 12'h3b2;
localparam PMPADDR3       = 12'h3b3;
localparam PMPADDR4       = 12'h3b4;
localparam PMPADDR5       = 12'h3b5;
localparam PMPADDR6       = 12'h3b6;
localparam PMPADDR7       = 12'h3b7;
localparam PMPADDR8       = 12'h3b8;
localparam PMPADDR9       = 12'h3b9;
localparam PMPADDR10      = 12'h3ba;
localparam PMPADDR11      = 12'h3bb;
localparam PMPADDR12      = 12'h3bc;
localparam PMPADDR13      = 12'h3bd;
localparam PMPADDR14      = 12'h3be;
localparam PMPADDR15      = 12'h3bf;

localparam MSECCFG        = 12'h747;
localparam MSECCFGH       = 12'h757;

// Performance counters (RW)
localparam MCYCLE         = 12'hb00; // Raw cycles since start of day
localparam MINSTRET       = 12'hb02; // Instruction retire count since start of day
localparam MHPMCOUNTER3   = 12'hb03; // WARL (we tie to 0)
localparam MHPMCOUNTER4   = 12'hb04; // WARL (we tie to 0)
localparam MHPMCOUNTER5   = 12'hb05; // WARL (we tie to 0)
localparam MHPMCOUNTER6   = 12'hb06; // WARL (we tie to 0)
localparam MHPMCOUNTER7   = 12'hb07; // WARL (we tie to 0)
localparam MHPMCOUNTER8   = 12'hb08; // WARL (we tie to 0)
localparam MHPMCOUNTER9   = 12'hb09; // WARL (we tie to 0)
localparam MHPMCOUNTER10  = 12'hb0a; // WARL (we tie to 0)
localparam MHPMCOUNTER11  = 12'hb0b; // WARL (we tie to 0)
localparam MHPMCOUNTER12  = 12'hb0c; // WARL (we tie to 0)
localparam MHPMCOUNTER13  = 12'hb0d; // WARL (we tie to 0)
localparam MHPMCOUNTER14  = 12'hb0e; // WARL (we tie to 0)
localparam MHPMCOUNTER15  = 12'hb0f; // WARL (we tie to 0)
localparam MHPMCOUNTER16  = 12'hb10; // WARL (we tie to 0)
localparam MHPMCOUNTER17  = 12'hb11; // WARL (we tie to 0)
localparam MHPMCOUNTER18  = 12'hb12; // WARL (we tie to 0)
localparam MHPMCOUNTER19  = 12'hb13; // WARL (we tie to 0)
localparam MHPMCOUNTER20  = 12'hb14; // WARL (we tie to 0)
localparam MHPMCOUNTER21  = 12'hb15; // WARL (we tie to 0)
localparam MHPMCOUNTER22  = 12'hb16; // WARL (we tie to 0)
localparam MHPMCOUNTER23  = 12'hb17; // WARL (we tie to 0)
localparam MHPMCOUNTER24  = 12'hb18; // WARL (we tie to 0)
localparam MHPMCOUNTER25  = 12'hb19; // WARL (we tie to 0)
localparam MHPMCOUNTER26  = 12'hb1a; // WARL (we tie to 0)
localparam MHPMCOUNTER27  = 12'hb1b; // WARL (we tie to 0)
localparam MHPMCOUNTER28  = 12'hb1c; // WARL (we tie to 0)
localparam MHPMCOUNTER29  = 12'hb1d; // WARL (we tie to 0)
localparam MHPMCOUNTER30  = 12'hb1e; // WARL (we tie to 0)
localparam MHPMCOUNTER31  = 12'hb1f; // WARL (we tie to 0)

localparam MCYCLEH        = 12'hb80; // High halves of each counter
localparam MINSTRETH      = 12'hb82;
localparam MHPMCOUNTER3H  = 12'hb83;
localparam MHPMCOUNTER4H  = 12'hb84;
localparam MHPMCOUNTER5H  = 12'hb85;
localparam MHPMCOUNTER6H  = 12'hb86;
localparam MHPMCOUNTER7H  = 12'hb87;
localparam MHPMCOUNTER8H  = 12'hb88;
localparam MHPMCOUNTER9H  = 12'hb89;
localparam MHPMCOUNTER10H = 12'hb8a;
localparam MHPMCOUNTER11H = 12'hb8b;
localparam MHPMCOUNTER12H = 12'hb8c;
localparam MHPMCOUNTER13H = 12'hb8d;
localparam MHPMCOUNTER14H = 12'hb8e;
localparam MHPMCOUNTER15H = 12'hb8f;
localparam MHPMCOUNTER16H = 12'hb90;
localparam MHPMCOUNTER17H = 12'hb91;
localparam MHPMCOUNTER18H = 12'hb92;
localparam MHPMCOUNTER19H = 12'hb93;
localparam MHPMCOUNTER20H = 12'hb94;
localparam MHPMCOUNTER21H = 12'hb95;
localparam MHPMCOUNTER22H = 12'hb96;
localparam MHPMCOUNTER23H = 12'hb97;
localparam MHPMCOUNTER24H = 12'hb98;
localparam MHPMCOUNTER25H = 12'hb99;
localparam MHPMCOUNTER26H = 12'hb9a;
localparam MHPMCOUNTER27H = 12'hb9b;
localparam MHPMCOUNTER28H = 12'hb9c;
localparam MHPMCOUNTER29H = 12'hb9d;
localparam MHPMCOUNTER30H = 12'hb9e;
localparam MHPMCOUNTER31H = 12'hb9f;

localparam MCOUNTINHIBIT  = 12'h320; // Count inhibit register for mcycle/minstret
localparam MHPMEVENT3     = 12'h323; // WARL (we tie to 0)
localparam MHPMEVENT4     = 12'h324; // WARL (we tie to 0)
localparam MHPMEVENT5     = 12'h325; // WARL (we tie to 0)
localparam MHPMEVENT6     = 12'h326; // WARL (we tie to 0)
localparam MHPMEVENT7     = 12'h327; // WARL (we tie to 0)
localparam MHPMEVENT8     = 12'h328; // WARL (we tie to 0)
localparam MHPMEVENT9     = 12'h329; // WARL (we tie to 0)
localparam MHPMEVENT10    = 12'h32a; // WARL (we tie to 0)
localparam MHPMEVENT11    = 12'h32b; // WARL (we tie to 0)
localparam MHPMEVENT12    = 12'h32c; // WARL (we tie to 0)
localparam MHPMEVENT13    = 12'h32d; // WARL (we tie to 0)
localparam MHPMEVENT14    = 12'h32e; // WARL (we tie to 0)
localparam MHPMEVENT15    = 12'h32f; // WARL (we tie to 0)
localparam MHPMEVENT16    = 12'h330; // WARL (we tie to 0)
localparam MHPMEVENT17    = 12'h331; // WARL (we tie to 0)
localparam MHPMEVENT18    = 12'h332; // WARL (we tie to 0)
localparam MHPMEVENT19    = 12'h333; // WARL (we tie to 0)
localparam MHPMEVENT20    = 12'h334; // WARL (we tie to 0)
localparam MHPMEVENT21    = 12'h335; // WARL (we tie to 0)
localparam MHPMEVENT22    = 12'h336; // WARL (we tie to 0)
localparam MHPMEVENT23    = 12'h337; // WARL (we tie to 0)
localparam MHPMEVENT24    = 12'h338; // WARL (we tie to 0)
localparam MHPMEVENT25    = 12'h339; // WARL (we tie to 0)
localparam MHPMEVENT26    = 12'h33a; // WARL (we tie to 0)
localparam MHPMEVENT27    = 12'h33b; // WARL (we tie to 0)
localparam MHPMEVENT28    = 12'h33c; // WARL (we tie to 0)
localparam MHPMEVENT29    = 12'h33d; // WARL (we tie to 0)
localparam MHPMEVENT30    = 12'h33e; // WARL (we tie to 0)
localparam MHPMEVENT31    = 12'h33f; // WARL (we tie to 0)

// Other standard M-mode CSRs:
localparam MENVCFG        = 12'h30a;
localparam MENVCFGH       = 12'h31a;

// Custom M-mode CSRs:
localparam PMPCFGM0       = 12'hbd0; // Make PMP regions M-mode without locking
//                              bd1  // (reserved for >32 regions)

localparam MEIEA          = 12'hbe0; // External interrupt pending array
localparam MEIPA          = 12'hbe1; // External interrupt enable array
localparam MEIFA          = 12'hbe2; // External interrupt force array
localparam MEIPRA         = 12'hbe3; // External interrupt priority array
localparam MEINEXT        = 12'hbe4; // Next external interrupt
localparam MEICONTEXT     = 12'hbe5; // External interrupt context register

localparam MSLEEP         = 12'hbf0; // M-mode sleep control register
localparam H3MISA         = 12'hbf1; // Hazard3 M-mode ISA identification register

// ----------------------------------------------------------------------------
// U-mode CSRs

// Read-only aliases of M-mode counter CSRs:
localparam CYCLE          = 12'hc00;
localparam TIME           = 12'hc01;
localparam INSTRET        = 12'hc02;
localparam CYCLEH         = 12'hc80;
localparam TIMEH          = 12'hc81;
localparam INSTRETH       = 12'hc82;

// Custom U-mode CSRs
localparam SLEEP          = 12'h8f0; // U-mode subset of M-mode sleep control

// ----------------------------------------------------------------------------
// Trigger Module

localparam TSELECT       = 12'h7a0;
localparam TDATA1        = 12'h7a1;
localparam TDATA2        = 12'h7a2;
localparam TDATA3        = 12'h7a3;
localparam TINFO         = 12'h7a4;
localparam TCONTROL      = 12'h7a5;
localparam MCONTEXT      = 12'h7a8;

// ----------------------------------------------------------------------------
// D-mode CSRs

localparam DCSR           = 12'h7b0;
localparam DPC            = 12'h7b1;
localparam DMDATA0        = 12'hbff; // Custom read/write

generate
if (DEBUG_SUPPORT == 0) begin: no_triggers

// The instantiation of this block should already be stubbed out in core.v if
// there are no triggers, but we still get warnings for elaborating this
// module with zero triggers, so add a generate block here too.

always @ (*) cfg_rdata = {W_DATA{1'b0}};
assign break_any = 1'b0;
assign break_d_mode = 1'b0;

end else begin: have_triggers

localparam TINDEX_ICOUNT     = BREAKPOINT_TRIGGERS + 0;
localparam TINDEX_INTERRUPT  = BREAKPOINT_TRIGGERS + 1;
localparam TINDEX_EXCEPTION  = BREAKPOINT_TRIGGERS + 2;
localparam N_TRIGGERS        = BREAKPOINT_TRIGGERS + 3;

// If there are no breakpoints, we still have one dummy register (hardwired to
// zero) for Verilog wrangling purposes. It has no synthesis effect.
localparam N_BREAKPOINT_REGS = BREAKPOINT_TRIGGERS > 0 ? BREAKPOINT_TRIGGERS : 1;

// ----------------------------------------------------------------------------
// Configuration state

localparam W_TSELECT = $clog2(N_TRIGGERS);

reg [W_TSELECT-1:0] tselect;

// Note tdata1 and mcontrol are the same CSR. tdata1 refers to the universal
// fields (type/dmode) and mcontrol refers to those fields specific to
// type=2 (address/data match), the only trigger type we implement.

// State for instruction address match triggers (breakpoints).
reg              bp_tdata1_dmode  [0:N_BREAKPOINT_REGS-1];
reg              mcontrol_action  [0:N_BREAKPOINT_REGS-1];
reg              mcontrol_m       [0:N_BREAKPOINT_REGS-1];
reg              mcontrol_u       [0:N_BREAKPOINT_REGS-1];
reg              mcontrol_execute [0:N_BREAKPOINT_REGS-1];
reg [W_DATA-1:0] bp_tdata2        [0:N_BREAKPOINT_REGS-1];

// State for instruction count trigger
// (hardwired: count=1 dmode=0 action=0; Debug mode single step is already
// available via dcsr)
reg             icount_m;
reg             icount_u;

// State for interrupt trigger
// (hardwired: action=1; M-mode trap-on-trap is useless as you lose the
// original trap state)
reg             trigger_irq_m;
reg             trigger_irq_u;
reg             trigger_irq_dmode;
reg [15:0]      trigger_irq_cause;

localparam [15:0] IMPLEMENTED_IRQ_CAUSES = {
	4'h0, // reserved
	1'b1, // meip
	3'h0, // reserved or unimplemented
	1'b1, // mtip
	3'h0, // reserved or unimplemented
	1'b1, // msip
	3'h0  // reserved or unimplemented
};

// State for exception trigger
// (hardwired: action=1; M-mode trap-on-trap is useless as you lose the
// original trap state)
reg             trigger_exception_m;
reg             trigger_exception_u;
reg             trigger_exception_dmode;
reg [15:0]      trigger_exception_cause;

localparam [15:0] IMPLEMENTED_EXCEPTION_CAUSES = {
	4'h0,         // reserved
	1'b1,         // 11 -> ecall from M-mode
	2'h0,         // reserved or unimplemented
	|U_MODE,      // 8  -> ecall from U-mode
	1'b1,         // 7  -> store/AMO fault
	1'b1,         // 6  -> store/AMO align
	1'b1,         // 5  -> load fault
	1'b1,         // 4  -> load align
	1'b0,         // 3  -> breakpoint; seems useless and risky so disallow
	1'b1,         // 2  -> illegal opcode
	1'b1,         // 1  -> fetch fault
	~|EXTENSION_C // 0  -> fetch align (only when IALIGN is 32-bit)
};

// ----------------------------------------------------------------------------
// Configuration write port

localparam N_TRIGGERS_PADDED = 1 << $clog2(N_TRIGGERS);
wire [N_TRIGGERS_PADDED-1:0] tselect_match = {{N_TRIGGERS_PADDED-1{1'b0}}, 1'b1} << tselect;

always @ (posedge clk or negedge rst_n) begin: cfg_update
	integer i;
	if (!rst_n) begin

		tselect <= {W_TSELECT{1'b0}};

		icount_m <= 1'b0;
		icount_u <= 1'b0;

		trigger_irq_m <= 1'b0;
		trigger_irq_u <= 1'b0;
		trigger_irq_dmode <= 1'b0;
		trigger_irq_cause <= 16'h0;

		trigger_exception_m <= 1'b0;
		trigger_exception_u <= 1'b0;
		trigger_exception_dmode <= 1'b0;
		trigger_exception_cause <= 16'h0;

		for (i = 0; i < BREAKPOINT_TRIGGERS; i = i + 1) begin
			bp_tdata1_dmode[i] <= 1'b0;
			mcontrol_action[i] <= 1'b0;
			mcontrol_m[i] <= 1'b0;
			mcontrol_u[i] <= 1'b0;
			mcontrol_execute[i] <= 1'b0;
			bp_tdata2[i] <= {W_DATA{1'b0}};
		end

	end else begin

		if (cfg_wen && cfg_addr == TSELECT) begin

			tselect <= cfg_wdata[W_TSELECT-1:0];

		end else if (cfg_wen && cfg_addr == TDATA1) begin

			if (tselect_match[TINDEX_ICOUNT]) begin
				// This trigger does not implement a dmode bit, as Debug-mode
				// break on single-step is already provided by dcsr.step
				icount_m <= cfg_wdata[9];
				icount_u <= cfg_wdata[6] && |U_MODE;
			end
			if (tselect_match[TINDEX_INTERRUPT] && !(trigger_irq_dmode && !x_d_mode)) begin
				trigger_irq_dmode <= cfg_wdata[27];
				trigger_irq_m <= cfg_wdata[9];
				trigger_irq_u <= cfg_wdata[6] && |U_MODE;
			end
			if (tselect_match[TINDEX_EXCEPTION] && !(trigger_exception_dmode && !x_d_mode)) begin
				trigger_exception_dmode <= cfg_wdata[27];
				trigger_exception_m <= cfg_wdata[9];
				trigger_exception_u <= cfg_wdata[6] && |U_MODE;
			end
			for (i = 0; i < BREAKPOINT_TRIGGERS; i = i + 1) begin
				if (tselect_match[i] && !(bp_tdata1_dmode[i] && !x_d_mode)) begin
					if (x_d_mode) begin
						bp_tdata1_dmode[i] <= cfg_wdata[27];
					end
					mcontrol_action[i] <= cfg_wdata[12];
					mcontrol_m[i] <= cfg_wdata[6];
					mcontrol_u[i] <= cfg_wdata[3] && |U_MODE;
					mcontrol_execute[i] <= cfg_wdata[2];
				end
			end

		end else if (cfg_wen && cfg_addr == TDATA2) begin

			if (tselect_match[TINDEX_INTERRUPT] && !(trigger_irq_dmode && !x_d_mode)) begin
				trigger_irq_cause <= cfg_wdata[15:0] & IMPLEMENTED_IRQ_CAUSES;
			end
			if (tselect_match[TINDEX_EXCEPTION] && !(trigger_exception_dmode && !x_d_mode)) begin
				trigger_exception_cause <= cfg_wdata[15:0] & IMPLEMENTED_EXCEPTION_CAUSES;
			end
			for (i = 0; i < BREAKPOINT_TRIGGERS; i = i + 1) begin
				if (tselect_match[i] && !(bp_tdata1_dmode[i] && !x_d_mode)) begin
					bp_tdata2[i] <= cfg_wdata & {{W_ADDR-2{1'b1}}, |EXTENSION_C, 1'b0};
				end
			end

		end

		if ((x_d_mode || event_trap_enter) && break_m_step) begin
			// count field is hardwired, so the trigger is required to disable
			// itself by clearing its own enables
			icount_m <= 1'b0;
			icount_u <= 1'b0;
		end

		// With no breakpoints, there is still a dummy entry to avoid
		// `generate` spaghetti; tools complain about comb processes without
		// sensitivities etc, so just synchronously tie to 0:
		if (BREAKPOINT_TRIGGERS == 0) begin
			bp_tdata1_dmode[0] <= 1'b0;
			mcontrol_action[0] <= 1'b0;
			mcontrol_m[0] <= 1'b0;
			mcontrol_u[0] <= 1'b0;
			mcontrol_execute[0] <= 1'b0;
			bp_tdata2[0] <= {W_DATA{1'b0}};
		end

	end
end

// ----------------------------------------------------------------------------
// Configuration read port

reg [W_DATA-1:0] tdata1_rdata [0:N_TRIGGERS_PADDED-1];
reg [W_DATA-1:0] tdata2_rdata [0:N_TRIGGERS_PADDED-1];
reg [W_DATA-1:0] tinfo_rdata  [0:N_TRIGGERS_PADDED-1];

always @ (*) begin: generate_padded_rdata

	// Default for unimplemented triggers
	integer i;
	for (i = 0; i < N_TRIGGERS_PADDED; i = i + 1) begin
		tdata1_rdata[i] = {W_DATA{1'b0}};
		tdata2_rdata[i] = {W_DATA{1'b0}};
		tinfo_rdata[i]  = 32'd1 << 0; // type = 0, no trigger
	end

	// Breakpoints are the first n triggers
	for (i = 0; i < BREAKPOINT_TRIGGERS; i = i + 1) begin
		tdata1_rdata[i] = {
			4'h2,                              // type = address/data match
			bp_tdata1_dmode[i],
			6'h00,                             // maskmax = 0, exact match only
			1'b0,                              // hit = 0, not implemented
			1'b0,                              // select = 0, address match only
			1'b0,                              // timing = 0, trigger before execution
			2'h0,                              // sizelo = 0, unsized
			{3'h0, mcontrol_action[i]},        // action = 0/1, break to M-mode/D-mode
			1'b0,                              // chain = 0, chaining is useless for exact matches
			4'h0,                              // match = 0, exact match only
			mcontrol_m[i],
			1'b0,
			1'b0,                              // s = 0, no S-mode
			mcontrol_u[i],
			mcontrol_execute[i],
			1'b0,                              // store = 0, this is not a watchpoint
			1'b0                               // load = 0, this is not a watchpoint
		};
		tdata2_rdata[i] = bp_tdata2[i];
		tinfo_rdata[i] = 32'd1 << 2;           // type = 2, address/data match
	end

	// Instruction count trigger
	tdata1_rdata[TINDEX_ICOUNT] = {
		4'h3,                                  // type = instruction count
		1'b0,                                  // dmode = 0 (Debug mode already has dcsr.step)
		2'h0,                                  // reserved
		1'b0,                                  // hit = 0
		14'd1,                                 // count = 1, single-step only
		icount_m,
		1'b0,                                  // reserved
		1'b0,                                  // s = 0, no S-mode
		icount_u,
		6'h0                                   // action = 0, break to M-mode
	};
	tinfo_rdata[TINDEX_ICOUNT] = 32'd1 << 3;   // type = 3, instruction count

	// Interrupt trigger
	tdata1_rdata[TINDEX_INTERRUPT] = {
		4'h4,                                  // type = interrupt
		trigger_irq_dmode,
		1'b0,                                  // hit = 0
		16'h0,                                 // reserved
		trigger_irq_m,
		1'b0,                                  // reserved
		1'b0,                                  // s = 0, no S-mode
		trigger_irq_u,
		6'd1                                   // action = 1, break to Debug mode (if dmode=1)
	};
	tdata2_rdata[TINDEX_INTERRUPT] = {
		16'h0,
		trigger_irq_cause & IMPLEMENTED_IRQ_CAUSES
	};
	tinfo_rdata[TINDEX_INTERRUPT] = 32'd1 << 4;

	// Exception trigger
	tdata1_rdata[TINDEX_EXCEPTION] = {
		4'h5,                                  // type = exception
		trigger_exception_dmode,
		1'b0,                                  // hit = 0
		16'h0,                                 // reserved
		trigger_exception_m,
		1'b0,                                  // reserved
		1'b0,                                  // s = 0, no S-mode
		trigger_exception_u,
		6'd1                                   // action = 1, break to Debug mode (if dmode=1)
	};
	tdata2_rdata[TINDEX_EXCEPTION] = {
		16'h0,
		trigger_exception_cause & IMPLEMENTED_EXCEPTION_CAUSES
	};
	tinfo_rdata[TINDEX_EXCEPTION] = 32'd1 << 5;

end

always @ (*) begin
	cfg_rdata = {W_DATA{1'b0}};
	if (cfg_addr == TSELECT) begin
		cfg_rdata = {{W_DATA-W_TSELECT{1'b0}}, tselect};
	end else if (cfg_addr == TDATA1) begin
		cfg_rdata = tdata1_rdata[tselect];
	end else if (cfg_addr == TDATA2) begin
		cfg_rdata = tdata2_rdata[tselect];
	end else if (cfg_addr == TINFO) begin
		cfg_rdata = tinfo_rdata[tselect];
	end
end

// ----------------------------------------------------------------------------
// Interrupt/exception trigger logic

// Ignore tcontrol.mte as these triggers never target M-mode.
wire exception_trigger_match =
	!x_d_mode && trigger_exception_dmode &&
	(x_m_mode ? trigger_exception_m : trigger_exception_u) &&
	event_exception &&
	trigger_exception_cause[event_trap_cause] &&
	IMPLEMENTED_EXCEPTION_CAUSES[event_trap_cause];

wire interrupt_trigger_match =
	!x_d_mode && trigger_irq_dmode &&
	(x_m_mode ? trigger_irq_m : trigger_irq_u) &&
	event_interrupt &&
	trigger_irq_cause[event_trap_cause] &&
	IMPLEMENTED_IRQ_CAUSES[event_trap_cause];

// Asserted no later than the end of the aphase for the instruction fetch at
// mtvec. Tags the dphase of trap handler instruction fetches as containing
// breakpoints.
reg break_ie;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		break_ie <= 1'b0;
	end else begin
		break_ie <= !x_d_mode && (break_ie || (
			exception_trigger_match || interrupt_trigger_match
		));
	end
end

// ----------------------------------------------------------------------------
// Instruction count trigger logic (single-step under M-mode control)

wire step_break_enabled = trig_m_en && !x_d_mode && (
	x_m_mode ? icount_m : icount_u
);

reg break_on_step;

always @ (posedge clk or negedge rst_n) begin
	if (!rst_n) begin
		break_on_step <= 1'b0;
	end else begin
		// Note icount triggers differ from dcsr.step in that they ignore
		// exceptions, only triggering on retired instructions.
		break_on_step <= !(x_d_mode || event_trap_enter) && (break_on_step || (
			event_instr_ret && step_break_enabled
		));
	end
end

assign break_m_step = break_on_step;

// ----------------------------------------------------------------------------
// Breakpoint trigger logic

// To reduce the fanin of jump and load/store gating in stage X, the address
// lookup is in stage F (fetch data phase). We check *fetch addresses*, not
// program counter values. Fetches are always word-sized and word-aligned.
//
// To ensure it is safe to do this, non-debug-mode writes to the TDATA1 and
// TDATA2 CSRs cause a prefetch flush, to maintain write-to-fetch ordering.
//
// It's possible for different breakpoints to match different halfwords of the
// fetch word. The trigger unit must report both matches separately, because
// it is not known at this point where the instruction boundaries are (we
// don't have the instruction data yet).

wire [N_BREAKPOINT_REGS-1:0] breakpoint_enabled;
wire [N_BREAKPOINT_REGS-1:0] breakpoint_match;
wire [N_BREAKPOINT_REGS-1:0] want_d_mode_break;
wire [N_BREAKPOINT_REGS-1:0] want_m_mode_break;
wire [N_BREAKPOINT_REGS-1:0] want_d_mode_break_hw0;
wire [N_BREAKPOINT_REGS-1:0] want_d_mode_break_hw1;
wire [N_BREAKPOINT_REGS-1:0] want_m_mode_break_hw0;
wire [N_BREAKPOINT_REGS-1:0] want_m_mode_break_hw1;

genvar g;
for (g = 0; g < N_BREAKPOINT_REGS; g = g + 1) begin: match_pc
	// Detect tripped breakpoints
	assign breakpoint_enabled[g] = mcontrol_execute[g] && !fetch_d_mode && (
		fetch_m_mode ? mcontrol_m[g] : mcontrol_u[g]
	);
	assign breakpoint_match[g] = breakpoint_enabled[g] && fetch_addr == {bp_tdata2[g][W_DATA-1:2], 2'b00};
	// Decide the type of break implied by the trip
	assign want_d_mode_break[g] = breakpoint_match[g] &&  mcontrol_action[g] && bp_tdata1_dmode[g];
	assign want_m_mode_break[g] = breakpoint_match[g] && !mcontrol_action[g] && trig_m_en;
	// Report separately for each halfword, so the frontend can pass this
	// through the prefetch buffer. A breakpoint exception is taken when
	// the first halfword of an instruction (of any size) is flagged with
	// a breakpoint, implying an exact match.
	assign want_d_mode_break_hw0[g] = want_d_mode_break[g] && !bp_tdata2[g][1];
	assign want_d_mode_break_hw1[g] = want_d_mode_break[g] &&  bp_tdata2[g][1];
	assign want_m_mode_break_hw0[g] = want_m_mode_break[g] && !bp_tdata2[g][1];
	assign want_m_mode_break_hw1[g] = want_m_mode_break[g] &&  bp_tdata2[g][1];
end

// Break flags to frontend (tag the current fetch dphase as containing a breakpoint):

assign break_any    = {
	|want_m_mode_break_hw1 || |want_d_mode_break_hw1 || break_ie,
	|want_m_mode_break_hw0 || |want_d_mode_break_hw0 || break_ie
} & {2{BREAKPOINT_TRIGGERS > 0}};

assign break_d_mode = {
	|want_d_mode_break_hw1 || break_ie,
	|want_d_mode_break_hw0 || break_ie
} & {2{BREAKPOINT_TRIGGERS > 0}};

end
endgenerate

endmodule


`default_nettype wire

