module neorv32_cpu_alu_muldiv_3f29546453678b855931c174a97d6c0894b8f546
  (input  clk_i,
   input  rstn_i,
   input  \ctrl_i_ctrl_i[if_reset] ,
   input  \ctrl_i_ctrl_i[if_ready] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_cur] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_nxt] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_ret] ,
   input  \ctrl_i_ctrl_i[rf_wb_en] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs1] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs2] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rd] ,
   input  \ctrl_i_ctrl_i[rf_zero] ,
   input  [2:0] \ctrl_i_ctrl_i[alu_op] ,
   input  \ctrl_i_ctrl_i[alu_sub] ,
   input  \ctrl_i_ctrl_i[alu_opa_mux] ,
   input  \ctrl_i_ctrl_i[alu_opb_mux] ,
   input  \ctrl_i_ctrl_i[alu_unsigned] ,
   input  [31:0] \ctrl_i_ctrl_i[alu_imm] ,
   input  \ctrl_i_ctrl_i[alu_cp_alu] ,
   input  \ctrl_i_ctrl_i[alu_cp_cfu] ,
   input  \ctrl_i_ctrl_i[alu_cp_fpu] ,
   input  \ctrl_i_ctrl_i[lsu_req] ,
   input  \ctrl_i_ctrl_i[lsu_rd] ,
   input  \ctrl_i_ctrl_i[lsu_wr] ,
   input  \ctrl_i_ctrl_i[lsu_mo_en] ,
   input  \ctrl_i_ctrl_i[lsu_mi_en] ,
   input  \ctrl_i_ctrl_i[lsu_priv] ,
   input  \ctrl_i_ctrl_i[csr_we] ,
   input  \ctrl_i_ctrl_i[csr_re] ,
   input  [11:0] \ctrl_i_ctrl_i[csr_addr] ,
   input  [31:0] \ctrl_i_ctrl_i[csr_wdata] ,
   input  [10:0] \ctrl_i_ctrl_i[cnt_event] ,
   input  [2:0] \ctrl_i_ctrl_i[ir_funct3] ,
   input  [11:0] \ctrl_i_ctrl_i[ir_funct12] ,
   input  [6:0] \ctrl_i_ctrl_i[ir_opcode] ,
   input  [15:0] \ctrl_i_ctrl_i[ir_rvc] ,
   input  \ctrl_i_ctrl_i[cpu_priv] ,
   input  \ctrl_i_ctrl_i[cpu_trap] ,
   input  \ctrl_i_ctrl_i[cpu_sync_exc] ,
   input  \ctrl_i_ctrl_i[cpu_debug] ,
   input  [1:0] \ctrl_i_ctrl_i[cpu_fence] ,
   input  [31:0] rs1_i,
   input  [31:0] rs2_i,
   output [31:0] res_o,
   output valid_o);
  wire [263:0] n9764;
  wire valid_cmd;
  wire [9:0] ctrl;
  wire [194:0] div;
  wire [97:0] mul;
  wire n9768;
  wire n9769;
  wire n9770;
  wire [6:0] n9771;
  wire n9773;
  wire n9774;
  wire n9775;
  wire n9776;
  wire n9777;
  wire n9779;
  wire n9780;
  wire n9781;
  wire n9782;
  wire n9785;
  wire [1:0] n9792;
  wire [4:0] n9793;
  wire [4:0] n9795;
  wire n9796;
  wire n9805;
  wire n9807;
  wire n9809;
  wire n9810;
  wire n9811;
  wire n9812;
  wire n9813;
  wire n9814;
  wire n9815;
  wire n9816;
  wire n9817;
  wire [1:0] n9819;
  wire [1:0] n9820;
  wire [1:0] n9821;
  wire n9823;
  wire n9827;
  wire [1:0] n9831;
  wire [1:0] n9832;
  wire [1:0] n9833;
  reg [1:0] n9834;
  reg [4:0] n9835;
  reg n9836;
  wire [6:0] n9837;
  wire [6:0] n9842;
  wire [1:0] n9847;
  wire n9849;
  wire n9850;
  wire [2:0] n9853;
  wire n9855;
  wire [2:0] n9856;
  wire n9858;
  wire n9859;
  wire [2:0] n9860;
  wire n9862;
  wire n9863;
  wire [2:0] n9864;
  wire n9866;
  wire n9867;
  wire n9868;
  wire [2:0] n9871;
  wire n9873;
  wire [2:0] n9874;
  wire n9876;
  wire n9877;
  wire [2:0] n9878;
  wire n9880;
  wire n9881;
  wire n9882;
  wire n9885;
  wire n9886;
  wire n9887;
  wire n9888;
  wire n9891;
  wire n9892;
  wire n9893;
  wire n9896;
  wire n9899;
  wire [1:0] n9901;
  wire n9903;
  wire n9904;
  wire n9905;
  wire n9906;
  wire [32:0] n9907;
  wire [30:0] n9908;
  wire [63:0] n9909;
  wire [63:0] n9910;
  wire [63:0] n9911;
  wire [63:0] n9912;
  wire [63:0] n9913;
  wire n9921;
  wire n9922;
  wire n9923;
  wire n9924;
  wire n9925;
  wire n9926;
  wire [32:0] n9927;
  wire n9928;
  wire [1:0] n9929;
  wire n9931;
  wire n9932;
  wire n9933;
  wire [31:0] n9934;
  wire [32:0] n9935;
  wire [32:0] n9936;
  wire [31:0] n9937;
  wire [32:0] n9938;
  wire [32:0] n9939;
  wire [32:0] n9940;
  wire [31:0] n9941;
  wire [32:0] n9942;
  wire [32:0] n9943;
  wire n9946;
  wire n9952;
  wire n9954;
  wire n9959;
  wire n9960;
  wire [31:0] n9962;
  wire [31:0] n9963;
  wire n9965;
  wire n9970;
  wire n9971;
  wire [31:0] n9973;
  wire [31:0] n9974;
  wire [1:0] n9976;
  wire n9983;
  wire n9985;
  wire n9987;
  wire n9988;
  wire n9989;
  wire n9990;
  wire n9991;
  wire n9992;
  wire n9993;
  wire n9994;
  wire n9995;
  wire n9996;
  wire n9997;
  wire n9998;
  wire n9999;
  wire n10000;
  wire n10001;
  wire n10002;
  wire n10003;
  wire n10004;
  wire n10005;
  wire n10006;
  wire n10007;
  wire n10008;
  wire n10009;
  wire n10010;
  wire n10011;
  wire n10012;
  wire n10013;
  wire n10014;
  wire n10015;
  wire n10016;
  wire n10017;
  wire n10018;
  wire n10019;
  wire n10020;
  wire n10021;
  wire n10022;
  wire n10023;
  wire n10024;
  wire n10025;
  wire n10026;
  wire n10027;
  wire n10028;
  wire n10029;
  wire n10030;
  wire n10031;
  wire n10032;
  wire n10033;
  wire n10034;
  wire n10035;
  wire n10036;
  wire n10037;
  wire n10038;
  wire n10039;
  wire n10040;
  wire n10041;
  wire n10042;
  wire n10043;
  wire n10044;
  wire n10045;
  wire n10046;
  wire n10047;
  wire n10048;
  wire n10049;
  wire n10050;
  wire n10051;
  wire n10052;
  wire n10054;
  wire n10055;
  wire n10057;
  wire [1:0] n10059;
  reg n10060;
  wire [1:0] n10061;
  wire n10063;
  wire [1:0] n10064;
  wire n10066;
  wire n10067;
  wire [30:0] n10068;
  wire n10069;
  wire n10070;
  wire [31:0] n10071;
  wire n10072;
  wire n10073;
  wire [31:0] n10074;
  wire [30:0] n10075;
  wire n10076;
  wire [31:0] n10077;
  wire [31:0] n10078;
  wire [63:0] n10079;
  wire [63:0] n10080;
  wire [63:0] n10081;
  wire [96:0] n10082;
  wire [31:0] n10083;
  wire [31:0] n10084;
  wire [31:0] n10085;
  wire [63:0] n10086;
  wire [63:0] n10087;
  wire n10088;
  wire n10089;
  wire n10090;
  wire [96:0] n10091;
  wire [96:0] n10094;
  wire [30:0] n10097;
  wire [31:0] n10099;
  wire n10100;
  wire [32:0] n10101;
  wire [31:0] n10102;
  wire [32:0] n10104;
  wire [32:0] n10105;
  wire [31:0] n10106;
  wire [1:0] n10107;
  wire n10109;
  wire [31:0] n10110;
  wire [31:0] n10111;
  wire [31:0] n10112;
  wire [31:0] n10114;
  wire n10115;
  wire [31:0] n10116;
  wire [31:0] n10117;
  wire n10119;
  wire [2:0] n10120;
  wire [31:0] n10121;
  wire n10123;
  wire [31:0] n10124;
  wire n10126;
  wire n10128;
  wire n10129;
  wire n10131;
  wire n10132;
  wire [31:0] n10133;
  wire [1:0] n10134;
  reg [31:0] n10135;
  wire [31:0] n10137;
  reg n10140;
  reg [6:0] n10141;
  wire [9:0] n10142;
  reg [96:0] n10143;
  wire [194:0] n10144;
  reg [63:0] n10145;
  wire [97:0] n10146;
  assign res_o = n10137; //(module output)
  assign valid_o = n9850; //(module output)
  assign n9764 = {\ctrl_i_ctrl_i[cpu_fence] , \ctrl_i_ctrl_i[cpu_debug] , \ctrl_i_ctrl_i[cpu_sync_exc] , \ctrl_i_ctrl_i[cpu_trap] , \ctrl_i_ctrl_i[cpu_priv] , \ctrl_i_ctrl_i[ir_rvc] , \ctrl_i_ctrl_i[ir_opcode] , \ctrl_i_ctrl_i[ir_funct12] , \ctrl_i_ctrl_i[ir_funct3] , \ctrl_i_ctrl_i[cnt_event] , \ctrl_i_ctrl_i[csr_wdata] , \ctrl_i_ctrl_i[csr_addr] , \ctrl_i_ctrl_i[csr_re] , \ctrl_i_ctrl_i[csr_we] , \ctrl_i_ctrl_i[lsu_priv] , \ctrl_i_ctrl_i[lsu_mi_en] , \ctrl_i_ctrl_i[lsu_mo_en] , \ctrl_i_ctrl_i[lsu_wr] , \ctrl_i_ctrl_i[lsu_rd] , \ctrl_i_ctrl_i[lsu_req] , \ctrl_i_ctrl_i[alu_cp_fpu] , \ctrl_i_ctrl_i[alu_cp_cfu] , \ctrl_i_ctrl_i[alu_cp_alu] , \ctrl_i_ctrl_i[alu_imm] , \ctrl_i_ctrl_i[alu_unsigned] , \ctrl_i_ctrl_i[alu_opb_mux] , \ctrl_i_ctrl_i[alu_opa_mux] , \ctrl_i_ctrl_i[alu_sub] , \ctrl_i_ctrl_i[alu_op] , \ctrl_i_ctrl_i[rf_zero] , \ctrl_i_ctrl_i[rf_rd] , \ctrl_i_ctrl_i[rf_rs2] , \ctrl_i_ctrl_i[rf_rs1] , \ctrl_i_ctrl_i[rf_wb_en] , \ctrl_i_ctrl_i[pc_ret] , \ctrl_i_ctrl_i[pc_nxt] , \ctrl_i_ctrl_i[pc_cur] , \ctrl_i_ctrl_i[if_ready] , \ctrl_i_ctrl_i[if_reset] };
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:65:10  */
  assign valid_cmd = n9782; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:76:10  */
  assign ctrl = n10142; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:89:10  */
  assign div = n10144; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:97:10  */
  assign mul = n10146; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:103:33  */
  assign n9768 = n9764[154]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:103:72  */
  assign n9769 = n9764[240]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:103:51  */
  assign n9770 = n9769 & n9768;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:104:43  */
  assign n9771 = n9764[234:228]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:104:57  */
  assign n9773 = n9771 == 7'b0000001;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:103:83  */
  assign n9774 = n9773 & n9770;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:105:43  */
  assign n9775 = n9764[222]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:105:47  */
  assign n9776 = ~n9775;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:105:75  */
  assign n9777 = n9764[222]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:105:86  */
  assign n9779 = 1'b1 & n9777;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:105:54  */
  assign n9780 = n9776 | n9779;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:104:70  */
  assign n9781 = n9780 & n9774;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:103:20  */
  assign n9782 = n9781 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:112:16  */
  assign n9785 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:122:17  */
  assign n9792 = ctrl[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:126:55  */
  assign n9793 = ctrl[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:126:60  */
  assign n9795 = n9793 - 5'b00001;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:127:22  */
  assign n9796 = n9764[259]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9805 = ctrl[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9807 = 1'b0 | n9805;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9809 = ctrl[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9810 = n9807 | n9809;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9811 = ctrl[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9812 = n9810 | n9811;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9813 = ctrl[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9814 = n9812 | n9813;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9815 = ctrl[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9816 = n9814 | n9815;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:129:40  */
  assign n9817 = ~n9816;
  assign n9819 = ctrl[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:129:11  */
  assign n9820 = n9817 ? 2'b10 : n9819;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:127:11  */
  assign n9821 = n9796 ? 2'b00 : n9820;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:124:9  */
  assign n9823 = n9792 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:133:9  */
  assign n9827 = n9792 == 2'b10;
  assign n9831 = ctrl[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:140:11  */
  assign n9832 = valid_cmd ? 2'b01 : n9831;
  assign n9833 = {n9827, n9823};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:122:7  */
  always @*
    case (n9833)
      2'b10: n9834 = 2'b00;
      2'b01: n9834 = n9821;
      default: n9834 = n9832;
    endcase
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:122:7  */
  always @*
    case (n9833)
      2'b10: n9835 = 5'b11110;
      2'b01: n9835 = n9795;
      default: n9835 = 5'b11110;
    endcase
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:122:7  */
  always @*
    case (n9833)
      2'b10: n9836 = 1'b1;
      2'b01: n9836 = 1'b0;
      default: n9836 = 1'b0;
    endcase
  assign n9837 = {n9835, n9834};
  assign n9842 = {5'b00000, 2'b00};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:153:29  */
  assign n9847 = ctrl[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:153:35  */
  assign n9849 = n9847 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:153:18  */
  assign n9850 = n9849 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:156:39  */
  assign n9853 = n9764[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:156:49  */
  assign n9855 = n9853 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:156:73  */
  assign n9856 = n9764[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:156:83  */
  assign n9858 = n9856 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:156:62  */
  assign n9859 = n9855 | n9858;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:157:39  */
  assign n9860 = n9764[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:157:49  */
  assign n9862 = n9860 == 3'b100;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:156:98  */
  assign n9863 = n9859 | n9862;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:157:73  */
  assign n9864 = n9764[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:157:83  */
  assign n9866 = n9864 == 3'b110;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:157:62  */
  assign n9867 = n9863 | n9866;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:156:26  */
  assign n9868 = n9867 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:158:39  */
  assign n9871 = n9764[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:158:49  */
  assign n9873 = n9871 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:159:39  */
  assign n9874 = n9764[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:159:49  */
  assign n9876 = n9874 == 3'b100;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:158:62  */
  assign n9877 = n9873 | n9876;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:159:73  */
  assign n9878 = n9764[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:159:83  */
  assign n9880 = n9878 == 3'b110;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:159:62  */
  assign n9881 = n9877 | n9880;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:158:26  */
  assign n9882 = n9881 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:162:64  */
  assign n9885 = n9764[222]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:162:68  */
  assign n9886 = ~n9885;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:162:43  */
  assign n9887 = n9886 & valid_cmd;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:162:20  */
  assign n9888 = n9887 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:163:64  */
  assign n9891 = n9764[222]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:163:43  */
  assign n9892 = n9891 & valid_cmd;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:163:20  */
  assign n9893 = n9892 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:196:18  */
  assign n9896 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:199:17  */
  assign n9899 = mul[0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:202:21  */
  assign n9901 = ctrl[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:202:27  */
  assign n9903 = n9901 != 2'b00;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:202:59  */
  assign n9904 = n9764[222]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:202:63  */
  assign n9905 = ~n9904;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:202:38  */
  assign n9906 = n9905 & n9903;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:203:43  */
  assign n9907 = mul[97:65]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:204:43  */
  assign n9908 = mul[32:2]; // extract
  assign n9909 = {n9907, n9908};
  assign n9910 = mul[64:1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:202:9  */
  assign n9911 = n9906 ? n9909 : n9910;
  assign n9912 = {32'b00000000000000000000000000000000, rs1_i};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:199:9  */
  assign n9913 = n9899 ? n9912 : n9911;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:214:24  */
  assign n9921 = mul[64]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:214:48  */
  assign n9922 = ctrl[8]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:214:39  */
  assign n9923 = n9921 & n9922;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:215:32  */
  assign n9924 = rs2_i[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:215:54  */
  assign n9925 = ctrl[8]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:215:45  */
  assign n9926 = n9924 & n9925;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:215:66  */
  assign n9927 = {n9926, rs2_i};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:216:18  */
  assign n9928 = mul[1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:217:18  */
  assign n9929 = ctrl[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:217:24  */
  assign n9931 = n9929 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:217:44  */
  assign n9932 = ctrl[7]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:217:34  */
  assign n9933 = n9932 & n9931;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:218:65  */
  assign n9934 = mul[64:33]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:218:56  */
  assign n9935 = {n9923, n9934};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:218:81  */
  assign n9936 = n9935 - n9927;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:220:65  */
  assign n9937 = mul[64:33]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:220:56  */
  assign n9938 = {n9923, n9937};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:220:81  */
  assign n9939 = n9938 + n9927;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:217:9  */
  assign n9940 = n9933 ? n9936 : n9939;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:223:36  */
  assign n9941 = mul[64:33]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:223:27  */
  assign n9942 = {n9923, n9941};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:216:7  */
  assign n9943 = n9928 ? n9940 : n9942;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:238:18  */
  assign n9946 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:244:17  */
  assign n9952 = div[0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:245:41  */
  assign n9954 = ctrl[8]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:47:14  */
  assign n9959 = rs2_i[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:47:34  */
  assign n9960 = n9954 & n9959;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:48:34  */
  assign n9962 = 32'b00000000000000000000000000000000 - rs2_i;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:47:5  */
  assign n9963 = n9960 ? n9962 : rs2_i;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:246:41  */
  assign n9965 = ctrl[7]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:47:14  */
  assign n9970 = rs1_i[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:47:34  */
  assign n9971 = n9965 & n9970;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:48:34  */
  assign n9973 = 32'b00000000000000000000000000000000 - rs1_i;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:47:5  */
  assign n9974 = n9971 ? n9973 : rs1_i;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:248:32  */
  assign n9976 = n9764[221:220]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9983 = rs2_i[31]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9985 = 1'b0 | n9983;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9987 = rs2_i[30]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9988 = n9985 | n9987;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9989 = rs2_i[29]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9990 = n9988 | n9989;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9991 = rs2_i[28]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9992 = n9990 | n9991;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9993 = rs2_i[27]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9994 = n9992 | n9993;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9995 = rs2_i[26]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9996 = n9994 | n9995;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9997 = rs2_i[25]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9998 = n9996 | n9997;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9999 = rs2_i[24]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10000 = n9998 | n9999;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10001 = rs2_i[23]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10002 = n10000 | n10001;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10003 = rs2_i[22]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10004 = n10002 | n10003;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10005 = rs2_i[21]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10006 = n10004 | n10005;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10007 = rs2_i[20]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10008 = n10006 | n10007;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10009 = rs2_i[19]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10010 = n10008 | n10009;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10011 = rs2_i[18]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10012 = n10010 | n10011;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10013 = rs2_i[17]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10014 = n10012 | n10013;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10015 = rs2_i[16]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10016 = n10014 | n10015;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10017 = rs2_i[15]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10018 = n10016 | n10017;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10019 = rs2_i[14]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10020 = n10018 | n10019;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10021 = rs2_i[13]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10022 = n10020 | n10021;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10023 = rs2_i[12]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10024 = n10022 | n10023;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10025 = rs2_i[11]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10026 = n10024 | n10025;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10027 = rs2_i[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10028 = n10026 | n10027;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10029 = rs2_i[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10030 = n10028 | n10029;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10031 = rs2_i[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10032 = n10030 | n10031;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10033 = rs2_i[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10034 = n10032 | n10033;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10035 = rs2_i[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10036 = n10034 | n10035;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10037 = rs2_i[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10038 = n10036 | n10037;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10039 = rs2_i[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10040 = n10038 | n10039;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10041 = rs2_i[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10042 = n10040 | n10041;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10043 = rs2_i[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10044 = n10042 | n10043;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10045 = rs2_i[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10046 = n10044 | n10045;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n10047 = rs2_i[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n10048 = n10046 | n10047;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:249:69  */
  assign n10049 = rs1_i[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:249:91  */
  assign n10050 = rs2_i[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:249:82  */
  assign n10051 = n10049 ^ n10050;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:249:59  */
  assign n10052 = n10048 & n10051;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:249:13  */
  assign n10054 = n9976 == 2'b00;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:250:45  */
  assign n10055 = rs1_i[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:250:13  */
  assign n10057 = n9976 == 2'b10;
  assign n10059 = {n10057, n10054};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:248:11  */
  always @*
    case (n10059)
      2'b10: n10060 = n10055;
      2'b01: n10060 = n10052;
      default: n10060 = 1'b0;
    endcase
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:253:21  */
  assign n10061 = ctrl[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:253:27  */
  assign n10063 = n10061 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:253:46  */
  assign n10064 = ctrl[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:253:52  */
  assign n10066 = n10064 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:253:37  */
  assign n10067 = n10063 | n10066;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:254:31  */
  assign n10068 = div[63:33]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:254:59  */
  assign n10069 = div[130]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:254:48  */
  assign n10070 = ~n10069;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:254:45  */
  assign n10071 = {n10068, n10070};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:255:22  */
  assign n10072 = div[130]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:255:27  */
  assign n10073 = ~n10072;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:256:32  */
  assign n10074 = div[129:98]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:258:33  */
  assign n10075 = div[95:65]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:258:57  */
  assign n10076 = div[64]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:258:47  */
  assign n10077 = {n10075, n10076};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:255:11  */
  assign n10078 = n10073 ? n10074 : n10077;
  assign n10079 = {n10078, n10071};
  assign n10080 = div[96:33]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:253:9  */
  assign n10081 = n10067 ? n10079 : n10080;
  assign n10082 = {n10060, 32'b00000000000000000000000000000000, n9974, n9963};
  assign n10083 = n10082[31:0]; // extract
  assign n10084 = div[32:1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:244:9  */
  assign n10085 = n9952 ? n10083 : n10084;
  assign n10086 = n10082[95:32]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:244:9  */
  assign n10087 = n9952 ? n10086 : n10081;
  assign n10088 = n10082[96]; // extract
  assign n10089 = div[97]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:244:9  */
  assign n10090 = n9952 ? n10088 : n10089;
  assign n10091 = {n10090, n10087, n10085};
  assign n10094 = {1'b0, 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:265:57  */
  assign n10097 = div[95:65]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:265:47  */
  assign n10099 = {1'b0, n10097};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:265:81  */
  assign n10100 = div[64]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:265:71  */
  assign n10101 = {n10099, n10100};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:265:108  */
  assign n10102 = div[32:1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:265:102  */
  assign n10104 = {1'b0, n10102};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:265:87  */
  assign n10105 = n10101 - n10104;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:268:22  */
  assign n10106 = div[64:33]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:268:49  */
  assign n10107 = n9764[222:221]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:268:62  */
  assign n10109 = n10107 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:268:27  */
  assign n10110 = n10109 ? n10106 : n10111;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:268:95  */
  assign n10111 = div[96:65]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:269:53  */
  assign n10112 = div[162:131]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:269:38  */
  assign n10114 = 32'b00000000000000000000000000000000 - n10112;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:269:71  */
  assign n10115 = div[97]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:269:61  */
  assign n10116 = n10115 ? n10114 : n10117;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:269:92  */
  assign n10117 = div[162:131]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:291:14  */
  assign n10119 = ctrl[9]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:292:19  */
  assign n10120 = n9764[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:294:27  */
  assign n10121 = mul[32:1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:293:9  */
  assign n10123 = n10120 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:296:27  */
  assign n10124 = mul[64:33]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:295:9  */
  assign n10126 = n10120 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:295:24  */
  assign n10128 = n10120 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:295:24  */
  assign n10129 = n10126 | n10128;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:295:38  */
  assign n10131 = n10120 == 3'b011;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:295:38  */
  assign n10132 = n10129 | n10131;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:298:24  */
  assign n10133 = div[194:163]; // extract
  assign n10134 = {n10132, n10123};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:292:7  */
  always @*
    case (n10134)
      2'b10: n10135 = n10124;
      2'b01: n10135 = n10121;
      default: n10135 = n10133;
    endcase
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:291:5  */
  assign n10137 = n10119 ? n10135 : 32'b00000000000000000000000000000000;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:116:5  */
  always @(posedge clk_i or posedge n9785)
    if (n9785)
      n10140 <= 1'b0;
    else
      n10140 <= n9836;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:116:5  */
  always @(posedge clk_i or posedge n9785)
    if (n9785)
      n10141 <= n9842;
    else
      n10141 <= n9837;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:112:5  */
  assign n10142 = {n10140, n9882, n9868, n10141};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:243:7  */
  always @(posedge clk_i or posedge n9946)
    if (n9946)
      n10143 <= n10094;
    else
      n10143 <= n10091;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:238:7  */
  assign n10144 = {n10116, n10110, n10105, n10143, n9893};
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:198:7  */
  always @(posedge clk_i or posedge n9896)
    if (n9896)
      n10145 <= 64'b0000000000000000000000000000000000000000000000000000000000000000;
    else
      n10145 <= n9913;
  /* ../../rtl/core/neorv32_cpu_alu_muldiv.vhd:196:7  */
  assign n10146 = {n9943, n10145, n9888};
endmodule

module neorv32_cpu_alu_shifter_5ba93c9db0cff93f52b521d7420e43f6eda2784f
  (input  clk_i,
   input  rstn_i,
   input  \ctrl_i_ctrl_i[if_reset] ,
   input  \ctrl_i_ctrl_i[if_ready] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_cur] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_nxt] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_ret] ,
   input  \ctrl_i_ctrl_i[rf_wb_en] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs1] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs2] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rd] ,
   input  \ctrl_i_ctrl_i[rf_zero] ,
   input  [2:0] \ctrl_i_ctrl_i[alu_op] ,
   input  \ctrl_i_ctrl_i[alu_sub] ,
   input  \ctrl_i_ctrl_i[alu_opa_mux] ,
   input  \ctrl_i_ctrl_i[alu_opb_mux] ,
   input  \ctrl_i_ctrl_i[alu_unsigned] ,
   input  [31:0] \ctrl_i_ctrl_i[alu_imm] ,
   input  \ctrl_i_ctrl_i[alu_cp_alu] ,
   input  \ctrl_i_ctrl_i[alu_cp_cfu] ,
   input  \ctrl_i_ctrl_i[alu_cp_fpu] ,
   input  \ctrl_i_ctrl_i[lsu_req] ,
   input  \ctrl_i_ctrl_i[lsu_rd] ,
   input  \ctrl_i_ctrl_i[lsu_wr] ,
   input  \ctrl_i_ctrl_i[lsu_mo_en] ,
   input  \ctrl_i_ctrl_i[lsu_mi_en] ,
   input  \ctrl_i_ctrl_i[lsu_priv] ,
   input  \ctrl_i_ctrl_i[csr_we] ,
   input  \ctrl_i_ctrl_i[csr_re] ,
   input  [11:0] \ctrl_i_ctrl_i[csr_addr] ,
   input  [31:0] \ctrl_i_ctrl_i[csr_wdata] ,
   input  [10:0] \ctrl_i_ctrl_i[cnt_event] ,
   input  [2:0] \ctrl_i_ctrl_i[ir_funct3] ,
   input  [11:0] \ctrl_i_ctrl_i[ir_funct12] ,
   input  [6:0] \ctrl_i_ctrl_i[ir_opcode] ,
   input  [15:0] \ctrl_i_ctrl_i[ir_rvc] ,
   input  \ctrl_i_ctrl_i[cpu_priv] ,
   input  \ctrl_i_ctrl_i[cpu_trap] ,
   input  \ctrl_i_ctrl_i[cpu_sync_exc] ,
   input  \ctrl_i_ctrl_i[cpu_debug] ,
   input  [1:0] \ctrl_i_ctrl_i[cpu_fence] ,
   input  [31:0] rs1_i,
   input  [4:0] shamt_i,
   output [31:0] res_o,
   output valid_o);
  wire [263:0] n9635;
  wire valid_cmd;
  wire [39:0] serial;
  wire n9639;
  wire [2:0] n9640;
  wire n9642;
  wire [6:0] n9643;
  wire n9645;
  wire n9646;
  wire [2:0] n9647;
  wire n9649;
  wire [6:0] n9650;
  wire n9652;
  wire n9653;
  wire n9654;
  wire [2:0] n9655;
  wire n9657;
  wire [6:0] n9658;
  wire n9660;
  wire n9661;
  wire n9662;
  wire n9663;
  wire n9664;
  wire n9667;
  wire n9674;
  wire n9675;
  wire n9676;
  wire n9678;
  wire n9679;
  wire n9680;
  wire n9681;
  wire n9682;
  wire n9683;
  wire n9691;
  wire n9693;
  wire n9695;
  wire n9696;
  wire n9697;
  wire n9698;
  wire n9699;
  wire n9700;
  wire n9701;
  wire n9702;
  wire [4:0] n9703;
  wire [4:0] n9705;
  wire n9706;
  wire n9707;
  wire [30:0] n9708;
  wire [31:0] n9710;
  wire n9711;
  wire n9712;
  wire n9713;
  wire [30:0] n9714;
  wire [31:0] n9715;
  wire [31:0] n9716;
  wire [36:0] n9717;
  wire [36:0] n9718;
  wire [36:0] n9719;
  wire [36:0] n9720;
  wire [36:0] n9721;
  wire [37:0] n9722;
  wire [37:0] n9727;
  wire n9738;
  wire n9740;
  wire n9742;
  wire n9743;
  wire n9744;
  wire n9745;
  wire n9746;
  wire n9747;
  wire n9748;
  wire n9749;
  wire n9750;
  wire n9751;
  wire [31:0] n9752;
  wire n9753;
  wire [31:0] n9754;
  reg [37:0] n9760;
  reg n9761;
  wire [39:0] n9762;
  assign res_o = n9754; //(module output)
  assign valid_o = n9751; //(module output)
  assign n9635 = {\ctrl_i_ctrl_i[cpu_fence] , \ctrl_i_ctrl_i[cpu_debug] , \ctrl_i_ctrl_i[cpu_sync_exc] , \ctrl_i_ctrl_i[cpu_trap] , \ctrl_i_ctrl_i[cpu_priv] , \ctrl_i_ctrl_i[ir_rvc] , \ctrl_i_ctrl_i[ir_opcode] , \ctrl_i_ctrl_i[ir_funct12] , \ctrl_i_ctrl_i[ir_funct3] , \ctrl_i_ctrl_i[cnt_event] , \ctrl_i_ctrl_i[csr_wdata] , \ctrl_i_ctrl_i[csr_addr] , \ctrl_i_ctrl_i[csr_re] , \ctrl_i_ctrl_i[csr_we] , \ctrl_i_ctrl_i[lsu_priv] , \ctrl_i_ctrl_i[lsu_mi_en] , \ctrl_i_ctrl_i[lsu_mo_en] , \ctrl_i_ctrl_i[lsu_wr] , \ctrl_i_ctrl_i[lsu_rd] , \ctrl_i_ctrl_i[lsu_req] , \ctrl_i_ctrl_i[alu_cp_fpu] , \ctrl_i_ctrl_i[alu_cp_cfu] , \ctrl_i_ctrl_i[alu_cp_alu] , \ctrl_i_ctrl_i[alu_imm] , \ctrl_i_ctrl_i[alu_unsigned] , \ctrl_i_ctrl_i[alu_opb_mux] , \ctrl_i_ctrl_i[alu_opa_mux] , \ctrl_i_ctrl_i[alu_sub] , \ctrl_i_ctrl_i[alu_op] , \ctrl_i_ctrl_i[rf_zero] , \ctrl_i_ctrl_i[rf_rd] , \ctrl_i_ctrl_i[rf_rs2] , \ctrl_i_ctrl_i[rf_rs1] , \ctrl_i_ctrl_i[rf_wb_en] , \ctrl_i_ctrl_i[pc_ret] , \ctrl_i_ctrl_i[pc_nxt] , \ctrl_i_ctrl_i[pc_cur] , \ctrl_i_ctrl_i[if_ready] , \ctrl_i_ctrl_i[if_reset] };
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:42:10  */
  assign valid_cmd = n9664; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:52:10  */
  assign serial = n9762; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:68:33  */
  assign n9639 = n9635[154]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:69:35  */
  assign n9640 = n9635[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:69:45  */
  assign n9642 = n9640 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:69:83  */
  assign n9643 = n9635[234:228]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:69:97  */
  assign n9645 = n9643 == 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:69:61  */
  assign n9646 = n9645 & n9642;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:70:35  */
  assign n9647 = n9635[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:70:45  */
  assign n9649 = n9647 == 3'b101;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:70:83  */
  assign n9650 = n9635[234:228]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:70:97  */
  assign n9652 = n9650 == 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:70:61  */
  assign n9653 = n9652 & n9649;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:69:111  */
  assign n9654 = n9646 | n9653;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:71:35  */
  assign n9655 = n9635[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:71:45  */
  assign n9657 = n9655 == 3'b101;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:71:83  */
  assign n9658 = n9635[234:228]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:71:97  */
  assign n9660 = n9658 == 7'b0100000;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:71:61  */
  assign n9661 = n9660 & n9657;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:70:111  */
  assign n9662 = n9654 | n9661;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:68:51  */
  assign n9663 = n9662 & n9639;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:68:20  */
  assign n9664 = n9663 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:82:18  */
  assign n9667 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:91:23  */
  assign n9674 = serial[1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:91:46  */
  assign n9675 = n9635[259]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:91:35  */
  assign n9676 = n9674 | n9675;
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n9678 = serial[0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:91:9  */
  assign n9679 = n9676 ? 1'b0 : n9678;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:89:9  */
  assign n9680 = valid_cmd ? 1'b1 : n9679;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:94:29  */
  assign n9681 = serial[0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:94:45  */
  assign n9682 = serial[1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:94:34  */
  assign n9683 = n9681 & n9682;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9691 = serial[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9693 = 1'b0 | n9691;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9695 = serial[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9696 = n9693 | n9695;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9697 = serial[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9698 = n9696 | n9697;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9699 = serial[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9700 = n9698 | n9699;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9701 = serial[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9702 = n9700 | n9701;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:100:59  */
  assign n9703 = serial[7:3]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:100:64  */
  assign n9705 = n9703 - 5'b00001;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:101:31  */
  assign n9706 = n9635[222]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:101:35  */
  assign n9707 = ~n9706;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:102:39  */
  assign n9708 = serial[38:8]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:102:69  */
  assign n9710 = {n9708, 1'b0};
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:104:40  */
  assign n9711 = serial[39]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:104:80  */
  assign n9712 = n9635[233]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:104:59  */
  assign n9713 = n9711 & n9712;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:104:99  */
  assign n9714 = serial[39:9]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:104:86  */
  assign n9715 = {n9713, n9714};
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:101:11  */
  assign n9716 = n9707 ? n9710 : n9715;
  assign n9717 = {n9716, n9705};
  assign n9718 = serial[39:3]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:99:9  */
  assign n9719 = n9702 ? n9717 : n9718;
  assign n9720 = {rs1_i, shamt_i};
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:96:9  */
  assign n9721 = valid_cmd ? n9720 : n9719;
  assign n9722 = {n9721, n9683};
  assign n9727 = {32'b00000000000000000000000000000000, 5'b00000, 1'b0};
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9738 = serial[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9740 = 1'b0 | n9738;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9742 = serial[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9743 = n9740 | n9742;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9744 = serial[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9745 = n9743 | n9744;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n9746 = serial[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n9747 = n9745 | n9746;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:111:20  */
  assign n9748 = ~n9747;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:112:27  */
  assign n9749 = serial[0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:112:43  */
  assign n9750 = serial[1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:112:32  */
  assign n9751 = n9749 & n9750;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:113:27  */
  assign n9752 = serial[39:8]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:113:45  */
  assign n9753 = serial[2]; // extract
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:113:32  */
  assign n9754 = n9753 ? n9752 : 32'b00000000000000000000000000000000;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:87:7  */
  always @(posedge clk_i or posedge n9667)
    if (n9667)
      n9760 <= n9727;
    else
      n9760 <= n9722;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:87:7  */
  always @(posedge clk_i or posedge n9667)
    if (n9667)
      n9761 <= 1'b0;
    else
      n9761 <= n9680;
  /* ../../rtl/core/neorv32_cpu_alu_shifter.vhd:82:7  */
  assign n9762 = {n9760, n9748, n9761};
endmodule

module neorv32_cpu_decompressor_5ba93c9db0cff93f52b521d7420e43f6eda2784f
  (input  [15:0] instr_i,
   output [31:0] instr_o);
  wire illegal;
  wire [31:0] decoded;
  wire [1:0] n9095;
  wire [2:0] n9096;
  wire [2:0] n9099;
  wire [4:0] n9101;
  wire [3:0] n9103;
  wire [5:0] n9105;
  wire [1:0] n9106;
  wire [7:0] n9107;
  wire n9108;
  wire [8:0] n9109;
  wire n9110;
  wire [9:0] n9111;
  wire [11:0] n9113;
  wire [7:0] n9114;
  wire n9116;
  wire n9119;
  wire n9121;
  wire n9123;
  wire [5:0] n9125;
  wire [2:0] n9126;
  wire [8:0] n9127;
  wire n9128;
  wire [9:0] n9129;
  wire [11:0] n9131;
  wire [2:0] n9133;
  wire [4:0] n9135;
  wire [2:0] n9136;
  wire [4:0] n9138;
  wire n9140;
  wire n9142;
  wire [5:0] n9144;
  wire n9145;
  wire [6:0] n9146;
  wire [1:0] n9147;
  wire n9148;
  wire [2:0] n9149;
  wire [4:0] n9151;
  wire [2:0] n9153;
  wire [4:0] n9155;
  wire [2:0] n9156;
  wire [4:0] n9158;
  wire n9160;
  wire n9162;
  wire [3:0] n9163;
  reg n9167;
  reg [6:0] n9169;
  reg [4:0] n9171;
  reg [2:0] n9173;
  reg [4:0] n9175;
  wire [4:0] n9176;
  wire [4:0] n9177;
  reg [4:0] n9179;
  wire [6:0] n9180;
  wire [6:0] n9181;
  reg [6:0] n9183;
  wire n9185;
  wire [2:0] n9186;
  wire n9187;
  wire n9188;
  wire [4:0] n9190;
  wire n9192;
  wire n9193;
  wire [1:0] n9194;
  wire [1:0] n9195;
  wire [3:0] n9196;
  wire n9197;
  wire [4:0] n9198;
  wire n9199;
  wire [5:0] n9200;
  wire n9201;
  wire [6:0] n9202;
  wire n9203;
  wire [7:0] n9204;
  wire [2:0] n9205;
  wire [10:0] n9206;
  wire n9207;
  wire [11:0] n9208;
  wire n9210;
  wire [3:0] n9216;
  wire [3:0] n9217;
  wire [7:0] n9218;
  wire [19:0] n9220;
  wire n9222;
  wire n9224;
  wire n9225;
  wire n9226;
  wire [2:0] n9228;
  wire [2:0] n9230;
  wire [4:0] n9232;
  wire n9235;
  wire [3:0] n9241;
  wire [1:0] n9243;
  wire [5:0] n9244;
  wire n9245;
  wire [6:0] n9246;
  wire [1:0] n9247;
  wire [1:0] n9248;
  wire [3:0] n9249;
  wire n9250;
  wire [4:0] n9251;
  wire n9253;
  wire n9255;
  wire n9256;
  wire [4:0] n9258;
  wire n9260;
  wire [3:0] n9266;
  wire [2:0] n9267;
  wire [6:0] n9268;
  wire [4:0] n9270;
  wire [11:0] n9271;
  wire n9273;
  wire [4:0] n9274;
  wire n9276;
  wire n9280;
  wire [2:0] n9286;
  wire [1:0] n9288;
  wire [4:0] n9289;
  wire n9290;
  wire [5:0] n9291;
  wire n9292;
  wire [6:0] n9293;
  wire n9294;
  wire [7:0] n9295;
  wire [11:0] n9297;
  wire [4:0] n9299;
  wire n9301;
  wire [3:0] n9307;
  wire [3:0] n9308;
  wire [3:0] n9309;
  wire [2:0] n9310;
  wire [14:0] n9311;
  wire [4:0] n9313;
  wire [19:0] n9314;
  wire [31:0] n9315;
  wire [31:0] n9316;
  wire [31:0] n9317;
  wire [4:0] n9318;
  wire n9320;
  wire n9321;
  wire n9322;
  wire n9323;
  wire n9326;
  wire n9328;
  wire [4:0] n9329;
  wire [4:0] n9330;
  wire n9332;
  wire [3:0] n9338;
  wire [2:0] n9339;
  wire [6:0] n9340;
  wire [4:0] n9342;
  wire [11:0] n9343;
  wire n9345;
  wire [2:0] n9346;
  wire [4:0] n9348;
  wire [2:0] n9349;
  wire [4:0] n9351;
  wire [2:0] n9352;
  wire [4:0] n9354;
  wire [1:0] n9355;
  wire n9356;
  wire [1:0] n9358;
  wire [6:0] n9360;
  wire [4:0] n9362;
  wire n9364;
  wire n9366;
  wire n9367;
  wire n9370;
  wire [3:0] n9376;
  wire [2:0] n9377;
  wire [6:0] n9378;
  wire [4:0] n9380;
  wire [11:0] n9381;
  wire n9383;
  wire [1:0] n9385;
  wire n9387;
  wire n9389;
  wire n9392;
  wire n9394;
  wire n9395;
  wire n9396;
  wire n9401;
  wire [2:0] n9403;
  wire [6:0] n9405;
  wire n9407;
  wire n9408;
  wire n9409;
  wire n9413;
  wire [2:0] n9415;
  wire [6:0] n9417;
  wire [2:0] n9418;
  reg n9419;
  reg [2:0] n9420;
  reg [6:0] n9421;
  wire [1:0] n9422;
  reg n9424;
  reg [6:0] n9425;
  reg [2:0] n9426;
  wire [4:0] n9427;
  reg [4:0] n9428;
  wire [6:0] n9429;
  reg [6:0] n9430;
  wire [4:0] n9431;
  reg n9433;
  wire [6:0] n9434;
  reg [6:0] n9435;
  wire [4:0] n9436;
  reg [4:0] n9437;
  wire [2:0] n9438;
  wire [2:0] n9439;
  reg [2:0] n9440;
  wire [4:0] n9441;
  wire [4:0] n9442;
  reg [4:0] n9443;
  wire [4:0] n9444;
  wire [4:0] n9445;
  wire [4:0] n9446;
  wire [4:0] n9447;
  reg [4:0] n9448;
  wire [6:0] n9449;
  wire [6:0] n9450;
  wire [6:0] n9451;
  wire [6:0] n9452;
  reg [6:0] n9453;
  wire n9455;
  wire [2:0] n9456;
  wire [4:0] n9457;
  wire [4:0] n9458;
  wire [4:0] n9461;
  wire n9463;
  wire [1:0] n9464;
  wire [5:0] n9466;
  wire n9467;
  wire [6:0] n9468;
  wire [2:0] n9469;
  wire [9:0] n9470;
  wire [11:0] n9472;
  wire [4:0] n9474;
  wire n9475;
  wire [4:0] n9476;
  wire n9478;
  wire n9479;
  wire n9482;
  wire n9484;
  wire n9486;
  wire n9487;
  wire [1:0] n9488;
  wire [5:0] n9490;
  wire n9491;
  wire [6:0] n9492;
  wire [2:0] n9493;
  wire [4:0] n9495;
  wire [4:0] n9497;
  wire n9498;
  wire n9501;
  wire n9503;
  wire n9505;
  wire n9506;
  wire n9507;
  wire n9508;
  wire [4:0] n9509;
  wire n9511;
  wire [4:0] n9513;
  wire [4:0] n9515;
  wire n9517;
  wire n9520;
  wire [4:0] n9522;
  wire [4:0] n9524;
  wire n9526;
  wire [24:0] n9527;
  wire [11:0] n9528;
  wire [11:0] n9529;
  wire [11:0] n9530;
  wire [2:0] n9531;
  wire [2:0] n9533;
  wire [4:0] n9534;
  wire [4:0] n9535;
  wire [4:0] n9536;
  wire [4:0] n9538;
  wire [4:0] n9539;
  wire n9541;
  wire [4:0] n9542;
  wire n9544;
  wire [4:0] n9547;
  wire [11:0] n9549;
  wire [6:0] n9550;
  wire [6:0] n9551;
  wire [4:0] n9552;
  wire [4:0] n9554;
  wire [4:0] n9556;
  wire [11:0] n9558;
  wire [4:0] n9560;
  wire [4:0] n9561;
  wire [4:0] n9562;
  wire [24:0] n9563;
  wire [11:0] n9564;
  wire [16:0] n9565;
  wire [11:0] n9566;
  wire [11:0] n9567;
  wire [2:0] n9568;
  wire [2:0] n9570;
  wire [9:0] n9571;
  wire [9:0] n9572;
  wire [9:0] n9573;
  wire [6:0] n9574;
  wire [6:0] n9576;
  wire n9578;
  wire [31:0] n9579;
  wire [24:0] n9580;
  wire [24:0] n9581;
  wire [24:0] n9582;
  wire [6:0] n9583;
  wire [6:0] n9585;
  wire n9587;
  wire [3:0] n9588;
  reg n9591;
  wire [6:0] n9592;
  reg [6:0] n9594;
  wire [4:0] n9595;
  reg [4:0] n9597;
  wire [2:0] n9598;
  reg [2:0] n9600;
  wire [4:0] n9601;
  reg [4:0] n9603;
  wire [4:0] n9604;
  wire [4:0] n9605;
  reg [4:0] n9607;
  wire [6:0] n9608;
  reg [6:0] n9610;
  wire [1:0] n9611;
  reg n9612;
  reg [6:0] n9614;
  reg [4:0] n9615;
  reg [2:0] n9616;
  reg [4:0] n9617;
  reg [4:0] n9618;
  reg [6:0] n9619;
  wire [29:0] n9627;
  wire n9628;
  wire n9629;
  wire n9630;
  wire [30:0] n9631;
  wire n9632;
  wire [31:0] n9633;
  wire [31:0] n9634;
  assign instr_o = n9633; //(module output)
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:52:10  */
  assign illegal = n9612; // (signal)
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:53:10  */
  assign decoded = n9634; // (signal)
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:66:17  */
  assign n9095 = instr_i[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:69:21  */
  assign n9096 = instr_i[15:13]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:75:84  */
  assign n9099 = instr_i[4:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:75:75  */
  assign n9101 = {2'b01, n9099};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:77:84  */
  assign n9103 = instr_i[10:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:77:75  */
  assign n9105 = {2'b00, n9103};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:77:107  */
  assign n9106 = instr_i[12:11]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:77:98  */
  assign n9107 = {n9105, n9106};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:77:131  */
  assign n9108 = instr_i[5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:77:122  */
  assign n9109 = {n9107, n9108};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:77:144  */
  assign n9110 = instr_i[6]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:77:135  */
  assign n9111 = {n9109, n9110};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:77:148  */
  assign n9113 = {n9111, 2'b00};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:78:24  */
  assign n9114 = instr_i[12:5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:78:38  */
  assign n9116 = n9114 == 8'b00000000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:78:13  */
  assign n9119 = n9116 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:71:11  */
  assign n9121 = n9096 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:85:87  */
  assign n9123 = instr_i[5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:85:78  */
  assign n9125 = {5'b00000, n9123};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:85:100  */
  assign n9126 = instr_i[12:10]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:85:91  */
  assign n9127 = {n9125, n9126};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:85:124  */
  assign n9128 = instr_i[6]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:85:115  */
  assign n9129 = {n9127, n9128};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:85:128  */
  assign n9131 = {n9129, 2'b00};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:87:84  */
  assign n9133 = instr_i[9:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:87:75  */
  assign n9135 = {2'b01, n9133};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:88:84  */
  assign n9136 = instr_i[4:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:88:75  */
  assign n9138 = {2'b01, n9136};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:82:11  */
  assign n9140 = n9096 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:93:87  */
  assign n9142 = instr_i[5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:93:78  */
  assign n9144 = {5'b00000, n9142};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:93:100  */
  assign n9145 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:93:91  */
  assign n9146 = {n9144, n9145};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:94:77  */
  assign n9147 = instr_i[11:10]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:94:101  */
  assign n9148 = instr_i[6]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:94:92  */
  assign n9149 = {n9147, n9148};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:94:105  */
  assign n9151 = {n9149, 2'b00};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:96:84  */
  assign n9153 = instr_i[9:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:96:75  */
  assign n9155 = {2'b01, n9153};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:97:84  */
  assign n9156 = instr_i[4:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:97:75  */
  assign n9158 = {2'b01, n9156};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:90:11  */
  assign n9160 = n9096 == 3'b110;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:99:11  */
  assign n9162 = n9096 == 3'b100;
  assign n9163 = {n9162, n9160, n9140, n9121};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:69:9  */
  always @*
    case (n9163)
      4'b1000: n9167 = 1'b1;
      4'b0100: n9167 = 1'b0;
      4'b0010: n9167 = 1'b0;
      4'b0001: n9167 = n9119;
      default: n9167 = 1'b1;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:69:9  */
  always @*
    case (n9163)
      4'b1000: n9169 = 7'b0000011;
      4'b0100: n9169 = 7'b0100011;
      4'b0010: n9169 = 7'b0000011;
      4'b0001: n9169 = 7'b0010011;
      default: n9169 = 7'b0000011;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:69:9  */
  always @*
    case (n9163)
      4'b1000: n9171 = 5'b00000;
      4'b0100: n9171 = n9151;
      4'b0010: n9171 = n9138;
      4'b0001: n9171 = n9101;
      default: n9171 = 5'b00000;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:69:9  */
  always @*
    case (n9163)
      4'b1000: n9173 = 3'b000;
      4'b0100: n9173 = 3'b010;
      4'b0010: n9173 = 3'b010;
      4'b0001: n9173 = 3'b000;
      default: n9173 = 3'b000;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:69:9  */
  always @*
    case (n9163)
      4'b1000: n9175 = 5'b00000;
      4'b0100: n9175 = n9155;
      4'b0010: n9175 = n9135;
      4'b0001: n9175 = 5'b00010;
      default: n9175 = 5'b00000;
    endcase
  assign n9176 = n9113[4:0]; // extract
  assign n9177 = n9131[4:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:69:9  */
  always @*
    case (n9163)
      4'b1000: n9179 = 5'b00000;
      4'b0100: n9179 = n9158;
      4'b0010: n9179 = n9177;
      4'b0001: n9179 = n9176;
      default: n9179 = 5'b00000;
    endcase
  assign n9180 = n9113[11:5]; // extract
  assign n9181 = n9131[11:5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:69:9  */
  always @*
    case (n9163)
      4'b1000: n9183 = 7'b0000000;
      4'b0100: n9183 = n9146;
      4'b0010: n9183 = n9181;
      4'b0001: n9183 = n9180;
      default: n9183 = 7'b0000000;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:68:7  */
  assign n9185 = n9095 == 2'b00;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:140:21  */
  assign n9186 = instr_i[15:13]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:144:91  */
  assign n9187 = instr_i[15]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:144:80  */
  assign n9188 = ~n9187;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:144:77  */
  assign n9190 = {4'b0000, n9188};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:77  */
  assign n9192 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:91  */
  assign n9193 = instr_i[8]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:82  */
  assign n9194 = {n9192, n9193};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:104  */
  assign n9195 = instr_i[10:9]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:95  */
  assign n9196 = {n9194, n9195};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:127  */
  assign n9197 = instr_i[6]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:118  */
  assign n9198 = {n9196, n9197};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:140  */
  assign n9199 = instr_i[7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:131  */
  assign n9200 = {n9198, n9199};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:147:77  */
  assign n9201 = instr_i[2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:146:144  */
  assign n9202 = {n9200, n9201};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:147:90  */
  assign n9203 = instr_i[11]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:147:81  */
  assign n9204 = {n9202, n9203};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:147:104  */
  assign n9205 = instr_i[5:3]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:147:95  */
  assign n9206 = {n9204, n9205};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:147:126  */
  assign n9207 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:147:117  */
  assign n9208 = {n9206, n9207};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:147:152  */
  assign n9210 = instr_i[12]; // extract
  assign n9216 = {n9210, n9210, n9210, n9210};
  assign n9217 = {n9210, n9210, n9210, n9210};
  assign n9218 = {n9216, n9217};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:147:131  */
  assign n9220 = {n9208, n9218};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:142:11  */
  assign n9222 = n9186 == 3'b101;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:142:22  */
  assign n9224 = n9186 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:142:22  */
  assign n9225 = n9222 | n9224;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:151:84  */
  assign n9226 = instr_i[13]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:151:75  */
  assign n9228 = {2'b00, n9226};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:153:84  */
  assign n9230 = instr_i[9:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:153:75  */
  assign n9232 = {2'b01, n9230};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:155:89  */
  assign n9235 = instr_i[12]; // extract
  assign n9241 = {n9235, n9235, n9235, n9235};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:155:107  */
  assign n9243 = instr_i[6:5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:155:98  */
  assign n9244 = {n9241, n9243};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:155:129  */
  assign n9245 = instr_i[2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:155:120  */
  assign n9246 = {n9244, n9245};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:156:77  */
  assign n9247 = instr_i[11:10]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:156:101  */
  assign n9248 = instr_i[4:3]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:156:92  */
  assign n9249 = {n9247, n9248};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:156:123  */
  assign n9250 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:156:114  */
  assign n9251 = {n9249, n9250};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:149:11  */
  assign n9253 = n9186 == 3'b110;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:149:22  */
  assign n9255 = n9186 == 3'b111;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:149:22  */
  assign n9256 = n9253 | n9255;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:163:77  */
  assign n9258 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:164:89  */
  assign n9260 = instr_i[12]; // extract
  assign n9266 = {n9260, n9260, n9260, n9260};
  assign n9267 = {n9260, n9260, n9260};
  assign n9268 = {n9266, n9267};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:164:106  */
  assign n9270 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:164:97  */
  assign n9271 = {n9268, n9270};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:158:11  */
  assign n9273 = n9186 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:168:24  */
  assign n9274 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:168:61  */
  assign n9276 = n9274 == 5'b00010;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:91  */
  assign n9280 = instr_i[12]; // extract
  assign n9286 = {n9280, n9280, n9280};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:108  */
  assign n9288 = instr_i[4:3]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:99  */
  assign n9289 = {n9286, n9288};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:130  */
  assign n9290 = instr_i[5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:121  */
  assign n9291 = {n9289, n9290};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:143  */
  assign n9292 = instr_i[2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:134  */
  assign n9293 = {n9291, n9292};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:156  */
  assign n9294 = instr_i[6]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:147  */
  assign n9295 = {n9293, n9294};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:173:160  */
  assign n9297 = {n9295, 4'b0000};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:176:79  */
  assign n9299 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:177:91  */
  assign n9301 = instr_i[12]; // extract
  assign n9307 = {n9301, n9301, n9301, n9301};
  assign n9308 = {n9301, n9301, n9301, n9301};
  assign n9309 = {n9301, n9301, n9301, n9301};
  assign n9310 = {n9301, n9301, n9301};
  assign n9311 = {n9307, n9308, n9309, n9310};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:177:109  */
  assign n9313 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:177:100  */
  assign n9314 = {n9311, n9313};
  assign n9315 = {n9314, n9299, 7'b0110111};
  assign n9316 = {n9297, 5'b00010, 3'b000, 5'b00010, 7'b0010011};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:168:13  */
  assign n9317 = n9276 ? n9316 : n9315;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:179:24  */
  assign n9318 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:179:37  */
  assign n9320 = n9318 == 5'b00000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:179:60  */
  assign n9321 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:179:65  */
  assign n9322 = ~n9321;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:179:48  */
  assign n9323 = n9322 & n9320;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:179:13  */
  assign n9326 = n9323 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:166:11  */
  assign n9328 = n9186 == 3'b011;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:187:77  */
  assign n9329 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:188:77  */
  assign n9330 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:189:89  */
  assign n9332 = instr_i[12]; // extract
  assign n9338 = {n9332, n9332, n9332, n9332};
  assign n9339 = {n9332, n9332, n9332};
  assign n9340 = {n9338, n9339};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:189:106  */
  assign n9342 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:189:97  */
  assign n9343 = {n9340, n9342};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:183:11  */
  assign n9345 = n9186 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:193:78  */
  assign n9346 = instr_i[9:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:193:69  */
  assign n9348 = {2'b01, n9346};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:194:78  */
  assign n9349 = instr_i[9:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:194:69  */
  assign n9351 = {2'b01, n9349};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:195:78  */
  assign n9352 = instr_i[4:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:195:69  */
  assign n9354 = {2'b01, n9352};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:196:25  */
  assign n9355 = instr_i[11:10]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:198:87  */
  assign n9356 = instr_i[10]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:198:78  */
  assign n9358 = {1'b0, n9356};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:198:92  */
  assign n9360 = {n9358, 5'b00000};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:201:81  */
  assign n9362 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:197:15  */
  assign n9364 = n9355 == 2'b00;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:197:25  */
  assign n9366 = n9355 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:197:25  */
  assign n9367 = n9364 | n9366;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:205:93  */
  assign n9370 = instr_i[12]; // extract
  assign n9376 = {n9370, n9370, n9370, n9370};
  assign n9377 = {n9370, n9370, n9370};
  assign n9378 = {n9376, n9377};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:205:110  */
  assign n9380 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:205:101  */
  assign n9381 = {n9378, n9380};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:202:15  */
  assign n9383 = n9355 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:208:29  */
  assign n9385 = instr_i[6:5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:212:39  */
  assign n9387 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:209:19  */
  assign n9389 = n9385 == 2'b00;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:216:39  */
  assign n9392 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:213:19  */
  assign n9394 = n9385 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:218:32  */
  assign n9395 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:218:37  */
  assign n9396 = ~n9395;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:218:21  */
  assign n9401 = n9396 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:218:21  */
  assign n9403 = n9396 ? 3'b110 : 3'b000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:218:21  */
  assign n9405 = n9396 ? 7'b0000000 : 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:217:19  */
  assign n9407 = n9385 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:228:32  */
  assign n9408 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:228:37  */
  assign n9409 = ~n9408;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:228:21  */
  assign n9413 = n9409 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:228:21  */
  assign n9415 = n9409 ? 3'b111 : 3'b000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:228:21  */
  assign n9417 = n9409 ? 7'b0000000 : 7'b0000000;
  assign n9418 = {n9407, n9394, n9389};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:208:17  */
  always @*
    case (n9418)
      3'b100: n9419 = n9401;
      3'b010: n9419 = n9392;
      3'b001: n9419 = n9387;
      default: n9419 = n9413;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:208:17  */
  always @*
    case (n9418)
      3'b100: n9420 = n9403;
      3'b010: n9420 = 3'b100;
      3'b001: n9420 = 3'b000;
      default: n9420 = n9415;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:208:17  */
  always @*
    case (n9418)
      3'b100: n9421 = n9405;
      3'b010: n9421 = 7'b0000000;
      3'b001: n9421 = 7'b0100000;
      default: n9421 = n9417;
    endcase
  assign n9422 = {n9383, n9367};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:196:13  */
  always @*
    case (n9422)
      2'b10: n9424 = 1'b0;
      2'b01: n9424 = 1'b0;
      default: n9424 = n9419;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:196:13  */
  always @*
    case (n9422)
      2'b10: n9425 = 7'b0010011;
      2'b01: n9425 = 7'b0010011;
      default: n9425 = 7'b0110011;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:196:13  */
  always @*
    case (n9422)
      2'b10: n9426 = 3'b111;
      2'b01: n9426 = 3'b101;
      default: n9426 = n9420;
    endcase
  assign n9427 = n9381[4:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:196:13  */
  always @*
    case (n9422)
      2'b10: n9428 = n9427;
      2'b01: n9428 = n9362;
      default: n9428 = n9354;
    endcase
  assign n9429 = n9381[11:5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:196:13  */
  always @*
    case (n9422)
      2'b10: n9430 = n9429;
      2'b01: n9430 = n9360;
      default: n9430 = n9421;
    endcase
  assign n9431 = {n9345, n9328, n9273, n9256, n9225};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:140:9  */
  always @*
    case (n9431)
      5'b10000: n9433 = 1'b0;
      5'b01000: n9433 = n9326;
      5'b00100: n9433 = 1'b0;
      5'b00010: n9433 = 1'b0;
      5'b00001: n9433 = 1'b0;
      default: n9433 = n9424;
    endcase
  assign n9434 = n9317[6:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:140:9  */
  always @*
    case (n9431)
      5'b10000: n9435 = 7'b0010011;
      5'b01000: n9435 = n9434;
      5'b00100: n9435 = 7'b0010011;
      5'b00010: n9435 = 7'b1100011;
      5'b00001: n9435 = 7'b1101111;
      default: n9435 = n9425;
    endcase
  assign n9436 = n9317[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:140:9  */
  always @*
    case (n9431)
      5'b10000: n9437 = n9330;
      5'b01000: n9437 = n9436;
      5'b00100: n9437 = n9258;
      5'b00010: n9437 = n9251;
      5'b00001: n9437 = n9190;
      default: n9437 = n9348;
    endcase
  assign n9438 = n9220[2:0]; // extract
  assign n9439 = n9317[14:12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:140:9  */
  always @*
    case (n9431)
      5'b10000: n9440 = 3'b000;
      5'b01000: n9440 = n9439;
      5'b00100: n9440 = 3'b000;
      5'b00010: n9440 = n9228;
      5'b00001: n9440 = n9438;
      default: n9440 = n9426;
    endcase
  assign n9441 = n9220[7:3]; // extract
  assign n9442 = n9317[19:15]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:140:9  */
  always @*
    case (n9431)
      5'b10000: n9443 = n9329;
      5'b01000: n9443 = n9442;
      5'b00100: n9443 = 5'b00000;
      5'b00010: n9443 = n9232;
      5'b00001: n9443 = n9441;
      default: n9443 = n9351;
    endcase
  assign n9444 = n9220[12:8]; // extract
  assign n9445 = n9271[4:0]; // extract
  assign n9446 = n9317[24:20]; // extract
  assign n9447 = n9343[4:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:140:9  */
  always @*
    case (n9431)
      5'b10000: n9448 = n9447;
      5'b01000: n9448 = n9446;
      5'b00100: n9448 = n9445;
      5'b00010: n9448 = 5'b00000;
      5'b00001: n9448 = n9444;
      default: n9448 = n9428;
    endcase
  assign n9449 = n9220[19:13]; // extract
  assign n9450 = n9271[11:5]; // extract
  assign n9451 = n9317[31:25]; // extract
  assign n9452 = n9343[11:5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:140:9  */
  always @*
    case (n9431)
      5'b10000: n9453 = n9452;
      5'b01000: n9453 = n9451;
      5'b00100: n9453 = n9450;
      5'b00010: n9453 = n9246;
      5'b00001: n9453 = n9449;
      default: n9453 = n9430;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:138:7  */
  assign n9455 = n9095 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:265:21  */
  assign n9456 = instr_i[15:13]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:270:77  */
  assign n9457 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:271:77  */
  assign n9458 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:274:77  */
  assign n9461 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:267:11  */
  assign n9463 = n9456 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:279:86  */
  assign n9464 = instr_i[3:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:279:77  */
  assign n9466 = {4'b0000, n9464};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:279:108  */
  assign n9467 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:279:99  */
  assign n9468 = {n9466, n9467};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:279:122  */
  assign n9469 = instr_i[6:4]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:279:113  */
  assign n9470 = {n9468, n9469};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:279:135  */
  assign n9472 = {n9470, 2'b00};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:282:77  */
  assign n9474 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:283:24  */
  assign n9475 = instr_i[13]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:284:24  */
  assign n9476 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:284:61  */
  assign n9478 = n9476 == 5'b00000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:283:49  */
  assign n9479 = n9475 | n9478;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:283:13  */
  assign n9482 = n9479 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:276:11  */
  assign n9484 = n9456 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:276:22  */
  assign n9486 = n9456 == 3'b011;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:276:22  */
  assign n9487 = n9484 | n9486;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:291:86  */
  assign n9488 = instr_i[8:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:291:77  */
  assign n9490 = {4'b0000, n9488};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:291:108  */
  assign n9491 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:291:99  */
  assign n9492 = {n9490, n9491};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:292:77  */
  assign n9493 = instr_i[11:9]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:292:91  */
  assign n9495 = {n9493, 2'b00};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:295:77  */
  assign n9497 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:296:24  */
  assign n9498 = instr_i[13]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:296:13  */
  assign n9501 = n9498 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:288:11  */
  assign n9503 = n9456 == 3'b110;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:288:22  */
  assign n9505 = n9456 == 3'b111;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:288:22  */
  assign n9506 = n9503 | n9505;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:302:24  */
  assign n9507 = instr_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:302:29  */
  assign n9508 = ~n9507;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:303:26  */
  assign n9509 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:303:65  */
  assign n9511 = n9509 == 5'b00000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:305:81  */
  assign n9513 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:307:28  */
  assign n9515 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:307:67  */
  assign n9517 = n9515 == 5'b00000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:307:17  */
  assign n9520 = n9517 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:313:81  */
  assign n9522 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:315:81  */
  assign n9524 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:303:15  */
  assign n9526 = n9511 ? n9520 : 1'b0;
  assign n9527 = {n9524, 5'b00000, 3'b000, n9522, 7'b0110011};
  assign n9528 = {5'b00000, 7'b1100111};
  assign n9529 = n9527[11:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:303:15  */
  assign n9530 = n9511 ? n9528 : n9529;
  assign n9531 = n9527[14:12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:303:15  */
  assign n9533 = n9511 ? 3'b000 : n9531;
  assign n9534 = n9527[19:15]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:303:15  */
  assign n9535 = n9511 ? n9513 : n9534;
  assign n9536 = n9527[24:20]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:303:15  */
  assign n9538 = n9511 ? 5'b00000 : n9536;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:318:26  */
  assign n9539 = instr_i[6:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:318:65  */
  assign n9541 = n9539 == 5'b00000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:319:28  */
  assign n9542 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:319:42  */
  assign n9544 = n9542 == 5'b00000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:324:83  */
  assign n9547 = instr_i[11:7]; // extract
  assign n9549 = {5'b00001, 7'b1100111};
  assign n9550 = n9549[6:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:319:17  */
  assign n9551 = n9544 ? 7'b1110011 : n9550;
  assign n9552 = n9549[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:319:17  */
  assign n9554 = n9544 ? 5'b00000 : n9552;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:319:17  */
  assign n9556 = n9544 ? 5'b00000 : n9547;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:319:17  */
  assign n9558 = n9544 ? 12'b000000000001 : 12'b000000000000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:330:81  */
  assign n9560 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:331:81  */
  assign n9561 = instr_i[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:332:81  */
  assign n9562 = instr_i[6:2]; // extract
  assign n9563 = {n9562, n9561, 3'b000, n9560, 7'b0110011};
  assign n9564 = {n9554, n9551};
  assign n9565 = {n9558, n9556};
  assign n9566 = n9563[11:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:318:15  */
  assign n9567 = n9541 ? n9564 : n9566;
  assign n9568 = n9563[14:12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:318:15  */
  assign n9570 = n9541 ? 3'b000 : n9568;
  assign n9571 = n9563[24:15]; // extract
  assign n9572 = n9565[9:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:318:15  */
  assign n9573 = n9541 ? n9572 : n9571;
  assign n9574 = n9565[16:10]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:318:15  */
  assign n9576 = n9541 ? n9574 : 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:302:13  */
  assign n9578 = n9508 ? n9526 : 1'b0;
  assign n9579 = {n9576, n9573, n9570, n9567};
  assign n9580 = {n9538, n9535, n9533, n9530};
  assign n9581 = n9579[24:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:302:13  */
  assign n9582 = n9508 ? n9580 : n9581;
  assign n9583 = n9579[31:25]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:302:13  */
  assign n9585 = n9508 ? 7'b0000000 : n9583;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:300:11  */
  assign n9587 = n9456 == 3'b100;
  assign n9588 = {n9587, n9506, n9487, n9463};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:265:9  */
  always @*
    case (n9588)
      4'b1000: n9591 = n9578;
      4'b0100: n9591 = n9501;
      4'b0010: n9591 = n9482;
      4'b0001: n9591 = 1'b0;
      default: n9591 = 1'b1;
    endcase
  assign n9592 = n9582[6:0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:265:9  */
  always @*
    case (n9588)
      4'b1000: n9594 = n9592;
      4'b0100: n9594 = 7'b0100011;
      4'b0010: n9594 = 7'b0000011;
      4'b0001: n9594 = 7'b0010011;
      default: n9594 = 7'b0000011;
    endcase
  assign n9595 = n9582[11:7]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:265:9  */
  always @*
    case (n9588)
      4'b1000: n9597 = n9595;
      4'b0100: n9597 = n9495;
      4'b0010: n9597 = n9474;
      4'b0001: n9597 = n9458;
      default: n9597 = 5'b00000;
    endcase
  assign n9598 = n9582[14:12]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:265:9  */
  always @*
    case (n9588)
      4'b1000: n9600 = n9598;
      4'b0100: n9600 = 3'b010;
      4'b0010: n9600 = 3'b010;
      4'b0001: n9600 = 3'b001;
      default: n9600 = 3'b000;
    endcase
  assign n9601 = n9582[19:15]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:265:9  */
  always @*
    case (n9588)
      4'b1000: n9603 = n9601;
      4'b0100: n9603 = 5'b00010;
      4'b0010: n9603 = 5'b00010;
      4'b0001: n9603 = n9457;
      default: n9603 = 5'b00000;
    endcase
  assign n9604 = n9472[4:0]; // extract
  assign n9605 = n9582[24:20]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:265:9  */
  always @*
    case (n9588)
      4'b1000: n9607 = n9605;
      4'b0100: n9607 = n9497;
      4'b0010: n9607 = n9604;
      4'b0001: n9607 = n9461;
      default: n9607 = 5'b00000;
    endcase
  assign n9608 = n9472[11:5]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:265:9  */
  always @*
    case (n9588)
      4'b1000: n9610 = n9585;
      4'b0100: n9610 = n9492;
      4'b0010: n9610 = n9608;
      4'b0001: n9610 = 7'b0000000;
      default: n9610 = 7'b0000000;
    endcase
  assign n9611 = {n9455, n9185};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:66:5  */
  always @*
    case (n9611)
      2'b10: n9612 = n9433;
      2'b01: n9612 = n9167;
      default: n9612 = n9591;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:66:5  */
  always @*
    case (n9611)
      2'b10: n9614 = n9435;
      2'b01: n9614 = n9169;
      default: n9614 = n9594;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:66:5  */
  always @*
    case (n9611)
      2'b10: n9615 = n9437;
      2'b01: n9615 = n9171;
      default: n9615 = n9597;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:66:5  */
  always @*
    case (n9611)
      2'b10: n9616 = n9440;
      2'b01: n9616 = n9173;
      default: n9616 = n9600;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:66:5  */
  always @*
    case (n9611)
      2'b10: n9617 = n9443;
      2'b01: n9617 = n9175;
      default: n9617 = n9603;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:66:5  */
  always @*
    case (n9611)
      2'b10: n9618 = n9448;
      2'b01: n9618 = n9179;
      default: n9618 = n9607;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:66:5  */
  always @*
    case (n9611)
      2'b10: n9619 = n9453;
      2'b01: n9619 = n9183;
      default: n9619 = n9610;
    endcase
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:346:21  */
  assign n9627 = decoded[31:2]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:346:45  */
  assign n9628 = decoded[1]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:346:54  */
  assign n9629 = ~illegal;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:346:49  */
  assign n9630 = n9628 & n9629;
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:346:35  */
  assign n9631 = {n9627, n9630};
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:346:77  */
  assign n9632 = decoded[0]; // extract
  /* ../../rtl/core/neorv32_cpu_decompressor.vhd:346:68  */
  assign n9633 = {n9631, n9632};
  assign n9634 = {n9619, n9618, n9617, n9616, n9615, n9614};
endmodule

module neorv32_cpu_frontend_ipb_1_17
  (input  clk_i,
   input  rstn_i,
   input  clear_i,
   input  [16:0] wdata_i,
   input  we_i,
   input  re_i,
   output free_o,
   output [16:0] rdata_o,
   output avail_o);
  wire [1:0] w_pnt;
  wire [1:0] r_pnt;
  wire match;
  wire n9031;
  wire [1:0] n9034;
  wire [1:0] n9035;
  wire [1:0] n9037;
  wire [1:0] n9039;
  wire [1:0] n9040;
  wire [1:0] n9042;
  wire n9051;
  wire n9052;
  wire n9053;
  wire n9054;
  wire n9057;
  wire n9058;
  wire n9059;
  wire n9060;
  wire n9061;
  wire n9064;
  wire n9065;
  wire n9066;
  wire n9067;
  wire n9068;
  wire n9072;
  wire n9081;
  reg [1:0] n9087;
  reg [1:0] n9088;
  wire [16:0] n9091; // mem_rd
  assign free_o = n9061; //(module output)
  assign rdata_o = n9091; //(module output)
  assign avail_o = n9068; //(module output)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:353:10  */
  assign w_pnt = n9087; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:353:17  */
  assign r_pnt = n9088; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:354:10  */
  assign match = n9054; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:366:16  */
  assign n9031 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:373:52  */
  assign n9034 = w_pnt + 2'b01;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:372:7  */
  assign n9035 = we_i ? n9034 : w_pnt;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:370:7  */
  assign n9037 = clear_i ? 2'b00 : n9035;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:378:52  */
  assign n9039 = r_pnt + 2'b01;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:377:7  */
  assign n9040 = re_i ? n9039 : r_pnt;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:375:7  */
  assign n9042 = clear_i ? 2'b00 : n9040;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:384:29  */
  assign n9051 = r_pnt[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:384:56  */
  assign n9052 = w_pnt[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:384:49  */
  assign n9053 = n9051 == n9052;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:384:18  */
  assign n9054 = n9053 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:385:29  */
  assign n9057 = r_pnt[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:385:46  */
  assign n9058 = w_pnt[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:385:38  */
  assign n9059 = n9057 != n9058;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:385:56  */
  assign n9060 = match & n9059;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:385:18  */
  assign n9061 = n9060 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:386:29  */
  assign n9064 = r_pnt[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:386:46  */
  assign n9065 = w_pnt[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:386:39  */
  assign n9066 = n9064 == n9065;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:386:56  */
  assign n9067 = match & n9066;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:386:18  */
  assign n9068 = n9067 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:394:38  */
  assign n9072 = w_pnt[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:400:43  */
  assign n9081 = r_pnt[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:369:5  */
  always @(posedge clk_i or posedge n9031)
    if (n9031)
      n9087 <= 2'b00;
    else
      n9087 <= n9037;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:369:5  */
  always @(posedge clk_i or posedge n9031)
    if (n9031)
      n9088 <= 2'b00;
    else
      n9088 <= n9042;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:400:18  */
  reg [16:0] ipb[1:0] ; // memory
  assign n9091 = ipb[n9081];
  always @(posedge clk_i)
    if (we_i)
      ipb[n9072] <= wdata_i;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:400:18  */
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:394:13  */
endmodule

module neorv32_clint_mtimecmp
  (input  clk_i,
   input  rstn_i,
   input  [63:0] mtime_i,
   input  [1:0] we_i,
   input  [1:0] re_i,
   input  [31:0] wdata_i,
   output [31:0] rdata_o,
   output mti_o);
  wire [63:0] mtimecmp_q;
  wire cmp_lo_eq;
  wire cmp_lo_gt;
  wire cmp_lo_ge;
  wire cmp_hi_eq;
  wire cmp_hi_gt;
  wire n8967;
  wire n8969;
  wire [31:0] n8970;
  wire [31:0] n8971;
  wire n8972;
  wire [31:0] n8973;
  wire [31:0] n8974;
  wire [63:0] n8975;
  wire [31:0] n8980;
  wire n8981;
  wire [31:0] n8982;
  wire [31:0] n8983;
  wire n8984;
  wire [31:0] n8985;
  wire n8988;
  wire n8990;
  wire n8991;
  wire n8992;
  wire [31:0] n9001;
  wire [31:0] n9002;
  wire n9003;
  wire n9004;
  wire [31:0] n9007;
  wire [31:0] n9008;
  wire n9009;
  wire n9010;
  wire [31:0] n9013;
  wire [31:0] n9014;
  wire n9015;
  wire n9016;
  wire [31:0] n9019;
  wire [31:0] n9020;
  wire n9021;
  wire n9022;
  reg [63:0] n9024;
  reg n9025;
  reg n9026;
  assign rdata_o = n8982; //(module output)
  assign mti_o = n9026; //(module output)
  /* ../../rtl/core/neorv32_clint.vhd:224:10  */
  assign mtimecmp_q = n9024; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:225:10  */
  assign cmp_lo_eq = n9004; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:225:21  */
  assign cmp_lo_gt = n9010; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:225:32  */
  assign cmp_lo_ge = n9025; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:225:43  */
  assign cmp_hi_eq = n9016; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:225:54  */
  assign cmp_hi_gt = n9022; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:233:16  */
  assign n8967 = ~rstn_i;
  /* ../../rtl/core/neorv32_clint.vhd:236:15  */
  assign n8969 = we_i[0]; // extract
  assign n8970 = mtimecmp_q[31:0]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:236:7  */
  assign n8971 = n8969 ? wdata_i : n8970;
  /* ../../rtl/core/neorv32_clint.vhd:239:15  */
  assign n8972 = we_i[1]; // extract
  assign n8973 = mtimecmp_q[63:32]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:239:7  */
  assign n8974 = n8972 ? wdata_i : n8973;
  assign n8975 = {n8974, n8971};
  /* ../../rtl/core/neorv32_clint.vhd:246:24  */
  assign n8980 = mtimecmp_q[63:32]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:246:49  */
  assign n8981 = re_i[1]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:246:39  */
  assign n8982 = n8981 ? n8980 : n8985;
  /* ../../rtl/core/neorv32_clint.vhd:247:24  */
  assign n8983 = mtimecmp_q[31:0]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:247:49  */
  assign n8984 = re_i[0]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:246:60  */
  assign n8985 = n8984 ? n8983 : 32'b00000000000000000000000000000000;
  /* ../../rtl/core/neorv32_clint.vhd:254:16  */
  assign n8988 = ~rstn_i;
  /* ../../rtl/core/neorv32_clint.vhd:258:30  */
  assign n8990 = cmp_lo_gt | cmp_lo_eq;
  /* ../../rtl/core/neorv32_clint.vhd:259:44  */
  assign n8991 = cmp_hi_eq & cmp_lo_ge;
  /* ../../rtl/core/neorv32_clint.vhd:259:30  */
  assign n8992 = cmp_hi_gt | n8991;
  /* ../../rtl/core/neorv32_clint.vhd:264:42  */
  assign n9001 = mtime_i[31:0]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:264:79  */
  assign n9002 = mtimecmp_q[31:0]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:264:58  */
  assign n9003 = n9001 == n9002;
  /* ../../rtl/core/neorv32_clint.vhd:264:20  */
  assign n9004 = n9003 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_clint.vhd:265:42  */
  assign n9007 = mtime_i[31:0]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:265:79  */
  assign n9008 = mtimecmp_q[31:0]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:265:58  */
  assign n9009 = $unsigned(n9007) > $unsigned(n9008);
  /* ../../rtl/core/neorv32_clint.vhd:265:20  */
  assign n9010 = n9009 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_clint.vhd:266:42  */
  assign n9013 = mtime_i[63:32]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:266:79  */
  assign n9014 = mtimecmp_q[63:32]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:266:58  */
  assign n9015 = n9013 == n9014;
  /* ../../rtl/core/neorv32_clint.vhd:266:20  */
  assign n9016 = n9015 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_clint.vhd:267:42  */
  assign n9019 = mtime_i[63:32]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:267:79  */
  assign n9020 = mtimecmp_q[63:32]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:267:58  */
  assign n9021 = $unsigned(n9019) > $unsigned(n9020);
  /* ../../rtl/core/neorv32_clint.vhd:267:20  */
  assign n9022 = n9021 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_clint.vhd:235:5  */
  always @(posedge clk_i or posedge n8967)
    if (n8967)
      n9024 <= 64'b0000000000000000000000000000000000000000000000000000000000000000;
    else
      n9024 <= n8975;
  /* ../../rtl/core/neorv32_clint.vhd:257:5  */
  always @(posedge clk_i or posedge n8988)
    if (n8988)
      n9025 <= 1'b0;
    else
      n9025 <= n8990;
  /* ../../rtl/core/neorv32_clint.vhd:257:5  */
  always @(posedge clk_i or posedge n8988)
    if (n8988)
      n9026 <= 1'b0;
    else
      n9026 <= n8992;
endmodule

module neorv32_prim_cnt_64
  (input  clk_i,
   input  rstn_i,
   input  inc_i,
   input  [1:0] we_i,
   input  [31:0] data_i,
   input  oe_i,
   output [63:0] cnt_o);
  wire [63:0] count;
  wire carry;
  wire incen;
  wire [32:0] inc_lo;
  wire [32:0] inc_hi;
  wire n8926;
  wire n8928;
  wire [31:0] n8929;
  wire [31:0] n8930;
  wire n8931;
  wire n8932;
  wire [31:0] n8933;
  wire [31:0] n8934;
  wire [63:0] n8935;
  wire [31:0] n8946;
  wire [32:0] n8948;
  wire [32:0] n8949;
  wire [32:0] n8950;
  wire [31:0] n8951;
  wire [32:0] n8953;
  wire [32:0] n8954;
  wire [32:0] n8955;
  wire [63:0] n8958;
  reg [63:0] n8961;
  reg n8962;
  reg n8963;
  assign cnt_o = n8958; //(module output)
  /* ../../rtl/core/neorv32_prim.vhd:351:10  */
  assign count = n8961; // (signal)
  /* ../../rtl/core/neorv32_prim.vhd:352:10  */
  assign carry = n8962; // (signal)
  /* ../../rtl/core/neorv32_prim.vhd:352:17  */
  assign incen = n8963; // (signal)
  /* ../../rtl/core/neorv32_prim.vhd:353:10  */
  assign inc_lo = n8950; // (signal)
  /* ../../rtl/core/neorv32_prim.vhd:353:18  */
  assign inc_hi = n8955; // (signal)
  /* ../../rtl/core/neorv32_prim.vhd:361:16  */
  assign n8926 = ~rstn_i;
  /* ../../rtl/core/neorv32_prim.vhd:369:15  */
  assign n8928 = we_i[0]; // extract
  /* ../../rtl/core/neorv32_prim.vhd:372:37  */
  assign n8929 = inc_lo[31:0]; // extract
  /* ../../rtl/core/neorv32_prim.vhd:369:7  */
  assign n8930 = n8928 ? data_i : n8929;
  /* ../../rtl/core/neorv32_prim.vhd:375:25  */
  assign n8931 = inc_lo[32]; // extract
  /* ../../rtl/core/neorv32_prim.vhd:377:15  */
  assign n8932 = we_i[1]; // extract
  /* ../../rtl/core/neorv32_prim.vhd:380:38  */
  assign n8933 = inc_hi[31:0]; // extract
  /* ../../rtl/core/neorv32_prim.vhd:377:7  */
  assign n8934 = n8932 ? data_i : n8933;
  assign n8935 = {n8934, n8930};
  /* ../../rtl/core/neorv32_prim.vhd:386:51  */
  assign n8946 = count[31:0]; // extract
  /* ../../rtl/core/neorv32_prim.vhd:386:44  */
  assign n8948 = {1'b0, n8946};
  /* ../../rtl/core/neorv32_prim.vhd:386:67  */
  assign n8949 = {32'b0, incen};  //  uext
  /* ../../rtl/core/neorv32_prim.vhd:386:67  */
  assign n8950 = n8948 + n8949;
  /* ../../rtl/core/neorv32_prim.vhd:387:51  */
  assign n8951 = count[63:32]; // extract
  /* ../../rtl/core/neorv32_prim.vhd:387:44  */
  assign n8953 = {1'b0, n8951};
  /* ../../rtl/core/neorv32_prim.vhd:387:67  */
  assign n8954 = {32'b0, carry};  //  uext
  /* ../../rtl/core/neorv32_prim.vhd:387:67  */
  assign n8955 = n8953 + n8954;
  /* ../../rtl/core/neorv32_prim.vhd:394:5  */
  assign n8958 = oe_i ? count : 64'b0000000000000000000000000000000000000000000000000000000000000000;
  /* ../../rtl/core/neorv32_prim.vhd:365:5  */
  always @(posedge clk_i or posedge n8926)
    if (n8926)
      n8961 <= 64'b0000000000000000000000000000000000000000000000000000000000000000;
    else
      n8961 <= n8935;
  /* ../../rtl/core/neorv32_prim.vhd:365:5  */
  always @(posedge clk_i or posedge n8926)
    if (n8926)
      n8962 <= 1'b0;
    else
      n8962 <= n8931;
  /* ../../rtl/core/neorv32_prim.vhd:365:5  */
  always @(posedge clk_i or posedge n8926)
    if (n8926)
      n8963 <= 1'b0;
    else
      n8963 <= inc_i;
endmodule

module neorv32_bus_reg_9159cb8bcee7fcb95582f140960cdae72788d326
  (input  clk_i,
   input  rstn_i,
   input  [4:0] \host_req_i_host_req_i[meta] ,
   input  [31:0] \host_req_i_host_req_i[addr] ,
   input  [31:0] \host_req_i_host_req_i[data] ,
   input  [3:0] \host_req_i_host_req_i[ben] ,
   input  \host_req_i_host_req_i[stb] ,
   input  \host_req_i_host_req_i[rw] ,
   input  \host_req_i_host_req_i[amo] ,
   input  [3:0] \host_req_i_host_req_i[amoop] ,
   input  \host_req_i_host_req_i[burst] ,
   input  \host_req_i_host_req_i[lock] ,
   input  \device_rsp_i_device_rsp_i[ack] ,
   input  \device_rsp_i_device_rsp_i[err] ,
   input  [31:0] \device_rsp_i_device_rsp_i[data] ,
   output \host_rsp_o_host_rsp_o[ack] ,
   output \host_rsp_o_host_rsp_o[err] ,
   output [31:0] \host_rsp_o_host_rsp_o[data] ,
   output [4:0] \device_req_o_device_req_o[meta] ,
   output [31:0] \device_req_o_device_req_o[addr] ,
   output [31:0] \device_req_o_device_req_o[data] ,
   output [3:0] \device_req_o_device_req_o[ben] ,
   output \device_req_o_device_req_o[stb] ,
   output \device_req_o_device_req_o[rw] ,
   output \device_req_o_device_req_o[amo] ,
   output [3:0] \device_req_o_device_req_o[amoop] ,
   output \device_req_o_device_req_o[burst] ,
   output \device_req_o_device_req_o[lock] );
  wire [81:0] n8881;
  wire n8883;
  wire n8884;
  wire [31:0] n8885;
  wire [4:0] n8887;
  wire [31:0] n8888;
  wire [31:0] n8889;
  wire [3:0] n8890;
  wire n8891;
  wire n8892;
  wire n8893;
  wire [3:0] n8894;
  wire n8895;
  wire n8896;
  wire [33:0] n8897;
  wire n8899;
  wire n8901;
  wire [81:0] n8902;
  wire n8903;
  wire [72:0] n8905;
  wire n8906;
  wire [5:0] n8908;
  wire n8909;
  wire [81:0] n8910;
  wire n8916;
  reg [33:0] n8922;
  reg [81:0] n8923;
  assign \host_rsp_o_host_rsp_o[ack]  = n8883; //(module output)
  assign \host_rsp_o_host_rsp_o[err]  = n8884; //(module output)
  assign \host_rsp_o_host_rsp_o[data]  = n8885; //(module output)
  assign \device_req_o_device_req_o[meta]  = n8887; //(module output)
  assign \device_req_o_device_req_o[addr]  = n8888; //(module output)
  assign \device_req_o_device_req_o[data]  = n8889; //(module output)
  assign \device_req_o_device_req_o[ben]  = n8890; //(module output)
  assign \device_req_o_device_req_o[stb]  = n8891; //(module output)
  assign \device_req_o_device_req_o[rw]  = n8892; //(module output)
  assign \device_req_o_device_req_o[amo]  = n8893; //(module output)
  assign \device_req_o_device_req_o[amoop]  = n8894; //(module output)
  assign \device_req_o_device_req_o[burst]  = n8895; //(module output)
  assign \device_req_o_device_req_o[lock]  = n8896; //(module output)
  assign n8881 = {\host_req_i_host_req_i[lock] , \host_req_i_host_req_i[burst] , \host_req_i_host_req_i[amoop] , \host_req_i_host_req_i[amo] , \host_req_i_host_req_i[rw] , \host_req_i_host_req_i[stb] , \host_req_i_host_req_i[ben] , \host_req_i_host_req_i[data] , \host_req_i_host_req_i[addr] , \host_req_i_host_req_i[meta] };
  /* ../../rtl/core/neorv32_cpu_counters.vhd:54:18  */
  assign n8883 = n8922[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8884 = n8922[1]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8885 = n8922[33:2]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8887 = n8923[4:0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8888 = n8923[36:5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8889 = n8923[68:37]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8890 = n8923[72:69]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8891 = n8923[73]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8892 = n8923[74]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1280:14  */
  assign n8893 = n8923[75]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8894 = n8923[79:76]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8895 = n8923[80]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8896 = n8923[81]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8897 = {\device_rsp_i_device_rsp_i[data] , \device_rsp_i_device_rsp_i[err] , \device_rsp_i_device_rsp_i[ack] };
  /* ../../rtl/core/neorv32_bus.vhd:199:18  */
  assign n8899 = ~rstn_i;
  /* ../../rtl/core/neorv32_bus.vhd:202:24  */
  assign n8901 = n8881[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:202:9  */
  assign n8902 = n8901 ? n8881 : n8923;
  /* ../../rtl/core/neorv32_bus.vhd:206:42  */
  assign n8903 = n8881[73]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8905 = n8902[72:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:207:42  */
  assign n8906 = n8881[80]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8908 = n8902[79:74]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:208:42  */
  assign n8909 = n8881[81]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8910 = {n8909, n8906, n8908, n8903, n8905};
  /* ../../rtl/core/neorv32_bus.vhd:224:18  */
  assign n8916 = ~rstn_i;
  /* ../../rtl/core/neorv32_bus.vhd:226:7  */
  always @(posedge clk_i or posedge n8916)
    if (n8916)
      n8922 <= 34'b0000000000000000000000000000000000;
    else
      n8922 <= n8897;
  /* ../../rtl/core/neorv32_bus.vhd:201:7  */
  always @(posedge clk_i or posedge n8899)
    if (n8899)
      n8923 <= 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000;
    else
      n8923 <= n8910;
endmodule

module neorv32_bus_reg_1489f923c4dca729178b3e3233458550d8dddf29
  (input  clk_i,
   input  rstn_i,
   input  [4:0] \host_req_i_host_req_i[meta] ,
   input  [31:0] \host_req_i_host_req_i[addr] ,
   input  [31:0] \host_req_i_host_req_i[data] ,
   input  [3:0] \host_req_i_host_req_i[ben] ,
   input  \host_req_i_host_req_i[stb] ,
   input  \host_req_i_host_req_i[rw] ,
   input  \host_req_i_host_req_i[amo] ,
   input  [3:0] \host_req_i_host_req_i[amoop] ,
   input  \host_req_i_host_req_i[burst] ,
   input  \host_req_i_host_req_i[lock] ,
   input  \device_rsp_i_device_rsp_i[ack] ,
   input  \device_rsp_i_device_rsp_i[err] ,
   input  [31:0] \device_rsp_i_device_rsp_i[data] ,
   output \host_rsp_o_host_rsp_o[ack] ,
   output \host_rsp_o_host_rsp_o[err] ,
   output [31:0] \host_rsp_o_host_rsp_o[data] ,
   output [4:0] \device_req_o_device_req_o[meta] ,
   output [31:0] \device_req_o_device_req_o[addr] ,
   output [31:0] \device_req_o_device_req_o[data] ,
   output [3:0] \device_req_o_device_req_o[ben] ,
   output \device_req_o_device_req_o[stb] ,
   output \device_req_o_device_req_o[rw] ,
   output \device_req_o_device_req_o[amo] ,
   output [3:0] \device_req_o_device_req_o[amoop] ,
   output \device_req_o_device_req_o[burst] ,
   output \device_req_o_device_req_o[lock] );
  wire [81:0] n8864;
  wire n8866;
  wire n8867;
  wire [31:0] n8868;
  wire [4:0] n8870;
  wire [31:0] n8871;
  wire [31:0] n8872;
  wire [3:0] n8873;
  wire n8874;
  wire n8875;
  wire n8876;
  wire [3:0] n8877;
  wire n8878;
  wire n8879;
  wire [33:0] n8880;
  assign \host_rsp_o_host_rsp_o[ack]  = n8866; //(module output)
  assign \host_rsp_o_host_rsp_o[err]  = n8867; //(module output)
  assign \host_rsp_o_host_rsp_o[data]  = n8868; //(module output)
  assign \device_req_o_device_req_o[meta]  = n8870; //(module output)
  assign \device_req_o_device_req_o[addr]  = n8871; //(module output)
  assign \device_req_o_device_req_o[data]  = n8872; //(module output)
  assign \device_req_o_device_req_o[ben]  = n8873; //(module output)
  assign \device_req_o_device_req_o[stb]  = n8874; //(module output)
  assign \device_req_o_device_req_o[rw]  = n8875; //(module output)
  assign \device_req_o_device_req_o[amo]  = n8876; //(module output)
  assign \device_req_o_device_req_o[amoop]  = n8877; //(module output)
  assign \device_req_o_device_req_o[burst]  = n8878; //(module output)
  assign \device_req_o_device_req_o[lock]  = n8879; //(module output)
  assign n8864 = {\host_req_i_host_req_i[lock] , \host_req_i_host_req_i[burst] , \host_req_i_host_req_i[amoop] , \host_req_i_host_req_i[amo] , \host_req_i_host_req_i[rw] , \host_req_i_host_req_i[stb] , \host_req_i_host_req_i[ben] , \host_req_i_host_req_i[data] , \host_req_i_host_req_i[addr] , \host_req_i_host_req_i[meta] };
  assign n8866 = n8880[0]; // extract
  assign n8867 = n8880[1]; // extract
  assign n8868 = n8880[33:2]; // extract
  assign n8870 = n8864[4:0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1280:14  */
  assign n8871 = n8864[36:5]; // extract
  assign n8872 = n8864[68:37]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8873 = n8864[72:69]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8874 = n8864[73]; // extract
  assign n8875 = n8864[74]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8876 = n8864[75]; // extract
  assign n8877 = n8864[79:76]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1280:14  */
  assign n8878 = n8864[80]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:50:23  */
  assign n8879 = n8864[81]; // extract
  /* ../../rtl/core/neorv32_package.vhd:921:12  */
  assign n8880 = {\device_rsp_i_device_rsp_i[data] , \device_rsp_i_device_rsp_i[err] , \device_rsp_i_device_rsp_i[ack] };
endmodule

module neorv32_cpu_lsu_0_5ba93c9db0cff93f52b521d7420e43f6eda2784f
  (input  clk_i,
   input  rstn_i,
   input  \ctrl_i_ctrl_i[if_reset] ,
   input  \ctrl_i_ctrl_i[if_ready] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_cur] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_nxt] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_ret] ,
   input  \ctrl_i_ctrl_i[rf_wb_en] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs1] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs2] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rd] ,
   input  \ctrl_i_ctrl_i[rf_zero] ,
   input  [2:0] \ctrl_i_ctrl_i[alu_op] ,
   input  \ctrl_i_ctrl_i[alu_sub] ,
   input  \ctrl_i_ctrl_i[alu_opa_mux] ,
   input  \ctrl_i_ctrl_i[alu_opb_mux] ,
   input  \ctrl_i_ctrl_i[alu_unsigned] ,
   input  [31:0] \ctrl_i_ctrl_i[alu_imm] ,
   input  \ctrl_i_ctrl_i[alu_cp_alu] ,
   input  \ctrl_i_ctrl_i[alu_cp_cfu] ,
   input  \ctrl_i_ctrl_i[alu_cp_fpu] ,
   input  \ctrl_i_ctrl_i[lsu_req] ,
   input  \ctrl_i_ctrl_i[lsu_rd] ,
   input  \ctrl_i_ctrl_i[lsu_wr] ,
   input  \ctrl_i_ctrl_i[lsu_mo_en] ,
   input  \ctrl_i_ctrl_i[lsu_mi_en] ,
   input  \ctrl_i_ctrl_i[lsu_priv] ,
   input  \ctrl_i_ctrl_i[csr_we] ,
   input  \ctrl_i_ctrl_i[csr_re] ,
   input  [11:0] \ctrl_i_ctrl_i[csr_addr] ,
   input  [31:0] \ctrl_i_ctrl_i[csr_wdata] ,
   input  [10:0] \ctrl_i_ctrl_i[cnt_event] ,
   input  [2:0] \ctrl_i_ctrl_i[ir_funct3] ,
   input  [11:0] \ctrl_i_ctrl_i[ir_funct12] ,
   input  [6:0] \ctrl_i_ctrl_i[ir_opcode] ,
   input  [15:0] \ctrl_i_ctrl_i[ir_rvc] ,
   input  \ctrl_i_ctrl_i[cpu_priv] ,
   input  \ctrl_i_ctrl_i[cpu_trap] ,
   input  \ctrl_i_ctrl_i[cpu_sync_exc] ,
   input  \ctrl_i_ctrl_i[cpu_debug] ,
   input  [1:0] \ctrl_i_ctrl_i[cpu_fence] ,
   input  [31:0] addr_i,
   input  [31:0] wdata_i,
   input  pmp_fault_i,
   input  \dbus_rsp_i_dbus_rsp_i[ack] ,
   input  \dbus_rsp_i_dbus_rsp_i[err] ,
   input  [31:0] \dbus_rsp_i_dbus_rsp_i[data] ,
   output [31:0] rdata_o,
   output [31:0] mar_o,
   output wait_o,
   output [3:0] err_o,
   output [4:0] \dbus_req_o_dbus_req_o[meta] ,
   output [31:0] \dbus_req_o_dbus_req_o[addr] ,
   output [31:0] \dbus_req_o_dbus_req_o[data] ,
   output [3:0] \dbus_req_o_dbus_req_o[ben] ,
   output \dbus_req_o_dbus_req_o[stb] ,
   output \dbus_req_o_dbus_req_o[rw] ,
   output \dbus_req_o_dbus_req_o[amo] ,
   output [3:0] \dbus_req_o_dbus_req_o[amoop] ,
   output \dbus_req_o_dbus_req_o[burst] ,
   output \dbus_req_o_dbus_req_o[lock] );
  wire [263:0] n8554;
  wire [4:0] n8560;
  wire [31:0] n8561;
  wire [31:0] n8562;
  wire [3:0] n8563;
  wire n8564;
  wire n8565;
  wire n8566;
  wire [3:0] n8567;
  wire n8568;
  wire n8569;
  wire [33:0] n8570;
  wire [81:0] req;
  wire misalign;
  wire n8575;
  wire n8582;
  wire n8583;
  wire [2:0] n8585;
  wire n8586;
  wire [3:0] n8587;
  wire [4:0] n8589;
  wire [1:0] n8590;
  wire [7:0] n8591;
  wire [7:0] n8592;
  wire [15:0] n8593;
  wire [7:0] n8594;
  wire [23:0] n8595;
  wire [7:0] n8596;
  wire [31:0] n8597;
  wire n8598;
  wire n8599;
  wire n8600;
  wire n8601;
  wire n8602;
  wire n8603;
  wire n8604;
  wire n8605;
  wire n8606;
  wire n8607;
  wire n8608;
  wire n8609;
  wire n8610;
  wire n8611;
  wire n8612;
  wire n8613;
  wire n8615;
  wire [15:0] n8616;
  wire [15:0] n8617;
  wire [31:0] n8618;
  wire n8619;
  wire n8620;
  wire [1:0] n8621;
  wire n8622;
  wire n8623;
  wire [2:0] n8624;
  wire n8625;
  wire n8626;
  wire [3:0] n8627;
  wire n8628;
  wire n8630;
  localparam [3:0] n8631 = 4'b1111;
  wire n8632;
  wire n8633;
  wire n8634;
  wire [1:0] n8635;
  reg [31:0] n8636;
  wire n8637;
  wire n8638;
  reg n8639;
  wire n8640;
  wire n8641;
  reg n8642;
  wire n8643;
  wire n8644;
  reg n8645;
  wire n8646;
  wire n8647;
  reg n8648;
  reg n8650;
  wire n8651;
  wire [72:0] n8652;
  wire [72:0] n8663;
  wire n8670;
  wire n8671;
  wire n8672;
  wire n8673;
  wire n8674;
  wire [31:0] n8675;
  wire n8677;
  wire n8679;
  wire [1:0] n8680;
  wire [1:0] n8681;
  wire n8683;
  wire n8684;
  wire n8685;
  wire n8686;
  wire [3:0] n8692;
  wire [3:0] n8693;
  wire [3:0] n8694;
  wire [3:0] n8695;
  wire [3:0] n8696;
  wire [3:0] n8697;
  wire [15:0] n8698;
  wire [7:0] n8699;
  wire [23:0] n8700;
  wire [7:0] n8702;
  wire [31:0] n8703;
  wire n8705;
  wire n8707;
  wire n8708;
  wire n8709;
  wire n8710;
  wire [3:0] n8716;
  wire [3:0] n8717;
  wire [3:0] n8718;
  wire [3:0] n8719;
  wire [3:0] n8720;
  wire [3:0] n8721;
  wire [15:0] n8722;
  wire [7:0] n8723;
  wire [23:0] n8724;
  wire [7:0] n8726;
  wire [31:0] n8727;
  wire n8729;
  wire n8731;
  wire n8732;
  wire n8733;
  wire n8734;
  wire [3:0] n8740;
  wire [3:0] n8741;
  wire [3:0] n8742;
  wire [3:0] n8743;
  wire [3:0] n8744;
  wire [3:0] n8745;
  wire [15:0] n8746;
  wire [7:0] n8747;
  wire [23:0] n8748;
  wire [7:0] n8750;
  wire [31:0] n8751;
  wire n8753;
  wire n8755;
  wire n8756;
  wire n8757;
  wire n8758;
  wire [3:0] n8764;
  wire [3:0] n8765;
  wire [3:0] n8766;
  wire [3:0] n8767;
  wire [3:0] n8768;
  wire [3:0] n8769;
  wire [15:0] n8770;
  wire [7:0] n8771;
  wire [23:0] n8772;
  wire [7:0] n8774;
  wire [31:0] n8775;
  wire [2:0] n8776;
  reg [31:0] n8777;
  wire n8779;
  wire n8780;
  wire n8781;
  wire n8783;
  wire n8784;
  wire n8785;
  wire n8786;
  wire [3:0] n8792;
  wire [3:0] n8793;
  wire [3:0] n8794;
  wire [3:0] n8795;
  wire [15:0] n8796;
  wire [15:0] n8798;
  wire [31:0] n8799;
  wire n8801;
  wire n8802;
  wire n8803;
  wire n8804;
  wire [3:0] n8810;
  wire [3:0] n8811;
  wire [3:0] n8812;
  wire [3:0] n8813;
  wire [15:0] n8814;
  wire [15:0] n8816;
  wire [31:0] n8817;
  wire [31:0] n8818;
  wire n8820;
  wire [31:0] n8821;
  wire [1:0] n8822;
  reg [31:0] n8823;
  wire [31:0] n8825;
  wire n8831;
  wire n8832;
  wire n8833;
  wire n8834;
  wire n8835;
  wire n8836;
  wire n8837;
  wire n8838;
  wire n8839;
  wire n8840;
  wire n8841;
  wire n8842;
  wire n8843;
  wire n8844;
  wire n8845;
  wire n8846;
  wire n8847;
  wire n8848;
  wire n8849;
  wire n8850;
  wire n8851;
  wire n8852;
  wire n8853;
  wire n8854;
  reg n8855;
  wire [72:0] n8856;
  wire [72:0] n8857;
  reg [72:0] n8858;
  wire [81:0] n8859;
  wire n8860;
  reg n8861;
  reg [31:0] n8862;
  wire [3:0] n8863;
  assign rdata_o = n8862; //(module output)
  assign mar_o = n8675; //(module output)
  assign wait_o = n8832; //(module output)
  assign err_o = n8863; //(module output)
  assign \dbus_req_o_dbus_req_o[meta]  = n8560; //(module output)
  assign \dbus_req_o_dbus_req_o[addr]  = n8561; //(module output)
  assign \dbus_req_o_dbus_req_o[data]  = n8562; //(module output)
  assign \dbus_req_o_dbus_req_o[ben]  = n8563; //(module output)
  assign \dbus_req_o_dbus_req_o[stb]  = n8564; //(module output)
  assign \dbus_req_o_dbus_req_o[rw]  = n8565; //(module output)
  assign \dbus_req_o_dbus_req_o[amo]  = n8566; //(module output)
  assign \dbus_req_o_dbus_req_o[amoop]  = n8567; //(module output)
  assign \dbus_req_o_dbus_req_o[burst]  = n8568; //(module output)
  assign \dbus_req_o_dbus_req_o[lock]  = n8569; //(module output)
  assign n8554 = {\ctrl_i_ctrl_i[cpu_fence] , \ctrl_i_ctrl_i[cpu_debug] , \ctrl_i_ctrl_i[cpu_sync_exc] , \ctrl_i_ctrl_i[cpu_trap] , \ctrl_i_ctrl_i[cpu_priv] , \ctrl_i_ctrl_i[ir_rvc] , \ctrl_i_ctrl_i[ir_opcode] , \ctrl_i_ctrl_i[ir_funct12] , \ctrl_i_ctrl_i[ir_funct3] , \ctrl_i_ctrl_i[cnt_event] , \ctrl_i_ctrl_i[csr_wdata] , \ctrl_i_ctrl_i[csr_addr] , \ctrl_i_ctrl_i[csr_re] , \ctrl_i_ctrl_i[csr_we] , \ctrl_i_ctrl_i[lsu_priv] , \ctrl_i_ctrl_i[lsu_mi_en] , \ctrl_i_ctrl_i[lsu_mo_en] , \ctrl_i_ctrl_i[lsu_wr] , \ctrl_i_ctrl_i[lsu_rd] , \ctrl_i_ctrl_i[lsu_req] , \ctrl_i_ctrl_i[alu_cp_fpu] , \ctrl_i_ctrl_i[alu_cp_cfu] , \ctrl_i_ctrl_i[alu_cp_alu] , \ctrl_i_ctrl_i[alu_imm] , \ctrl_i_ctrl_i[alu_unsigned] , \ctrl_i_ctrl_i[alu_opb_mux] , \ctrl_i_ctrl_i[alu_opa_mux] , \ctrl_i_ctrl_i[alu_sub] , \ctrl_i_ctrl_i[alu_op] , \ctrl_i_ctrl_i[rf_zero] , \ctrl_i_ctrl_i[rf_rd] , \ctrl_i_ctrl_i[rf_rs2] , \ctrl_i_ctrl_i[rf_rs1] , \ctrl_i_ctrl_i[rf_wb_en] , \ctrl_i_ctrl_i[pc_ret] , \ctrl_i_ctrl_i[pc_nxt] , \ctrl_i_ctrl_i[pc_cur] , \ctrl_i_ctrl_i[if_ready] , \ctrl_i_ctrl_i[if_reset] };
  assign n8560 = req[4:0]; // extract
  assign n8561 = req[36:5]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8562 = req[68:37]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8563 = req[72:69]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8564 = req[73]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8565 = req[74]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8566 = req[75]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8567 = req[79:76]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8568 = req[80]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1216:14  */
  assign n8569 = req[81]; // extract
  assign n8570 = {\dbus_rsp_i_dbus_rsp_i[data] , \dbus_rsp_i_dbus_rsp_i[err] , \dbus_rsp_i_dbus_rsp_i[ack] };
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:44:10  */
  assign req = n8859; // (signal)
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:45:10  */
  assign misalign = n8861; // (signal)
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:100:16  */
  assign n8575 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:108:18  */
  assign n8582 = n8554[160]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:109:73  */
  assign n8583 = n8554[261]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:109:64  */
  assign n8585 = {2'b00, n8583};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:109:92  */
  assign n8586 = n8554[162]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:109:83  */
  assign n8587 = {n8585, n8586};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:109:101  */
  assign n8589 = {n8587, 1'b0};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:111:30  */
  assign n8590 = n8554[221:220]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:113:34  */
  assign n8591 = wdata_i[7:0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:113:56  */
  assign n8592 = wdata_i[7:0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:113:47  */
  assign n8593 = {n8591, n8592};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:113:78  */
  assign n8594 = wdata_i[7:0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:113:69  */
  assign n8595 = {n8593, n8594};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:113:100  */
  assign n8596 = wdata_i[7:0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:113:91  */
  assign n8597 = {n8595, n8596};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:114:38  */
  assign n8598 = addr_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:114:28  */
  assign n8599 = ~n8598;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:114:58  */
  assign n8600 = addr_i[0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:114:48  */
  assign n8601 = ~n8600;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:114:43  */
  assign n8602 = n8599 & n8601;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:115:38  */
  assign n8603 = addr_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:115:28  */
  assign n8604 = ~n8603;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:115:58  */
  assign n8605 = addr_i[0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:115:43  */
  assign n8606 = n8604 & n8605;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:116:38  */
  assign n8607 = addr_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:116:58  */
  assign n8608 = addr_i[0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:116:48  */
  assign n8609 = ~n8608;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:116:43  */
  assign n8610 = n8607 & n8609;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:117:38  */
  assign n8611 = addr_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:117:58  */
  assign n8612 = addr_i[0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:117:43  */
  assign n8613 = n8611 & n8612;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:112:11  */
  assign n8615 = n8590 == 2'b00;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:120:32  */
  assign n8616 = wdata_i[15:0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:120:55  */
  assign n8617 = wdata_i[15:0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:120:46  */
  assign n8618 = {n8616, n8617};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:121:31  */
  assign n8619 = addr_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:121:43  */
  assign n8620 = addr_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:121:35  */
  assign n8621 = {n8619, n8620};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:121:60  */
  assign n8622 = addr_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:121:50  */
  assign n8623 = ~n8622;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:121:47  */
  assign n8624 = {n8621, n8623};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:121:78  */
  assign n8625 = addr_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:121:68  */
  assign n8626 = ~n8625;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:121:65  */
  assign n8627 = {n8624, n8626};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:122:31  */
  assign n8628 = addr_i[0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:119:11  */
  assign n8630 = n8590 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:126:31  */
  assign n8632 = addr_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:126:44  */
  assign n8633 = addr_i[0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:126:35  */
  assign n8634 = n8632 | n8633;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:29  */
  assign n8635 = {n8630, n8615};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:111:9  */
  always @*
    case (n8635)
      2'b10: n8636 = n8618;
      2'b01: n8636 = n8597;
      default: n8636 = wdata_i;
    endcase
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8637 = n8627[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8638 = n8631[0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:111:9  */
  always @*
    case (n8635)
      2'b10: n8639 = n8637;
      2'b01: n8639 = n8602;
      default: n8639 = n8638;
    endcase
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8640 = n8627[1]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8641 = n8631[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:111:9  */
  always @*
    case (n8635)
      2'b10: n8642 = n8640;
      2'b01: n8642 = n8606;
      default: n8642 = n8641;
    endcase
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:29  */
  assign n8643 = n8627[2]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8644 = n8631[2]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:111:9  */
  always @*
    case (n8635)
      2'b10: n8645 = n8643;
      2'b01: n8645 = n8610;
      default: n8645 = n8644;
    endcase
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8646 = n8627[3]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8647 = n8631[3]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:111:9  */
  always @*
    case (n8635)
      2'b10: n8648 = n8646;
      2'b01: n8648 = n8613;
      default: n8648 = n8647;
    endcase
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:111:9  */
  always @*
    case (n8635)
      2'b10: n8650 = n8628;
      2'b01: n8650 = 1'b0;
      default: n8650 = n8634;
    endcase
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:131:28  */
  assign n8651 = n8554[159]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8652 = {n8648, n8645, n8642, n8639, n8636, addr_i, n8589};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8663 = {4'b0000, 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 5'b00000};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:138:24  */
  assign n8670 = n8554[157]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:138:37  */
  assign n8671 = ~misalign;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:138:32  */
  assign n8672 = n8670 & n8671;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:138:56  */
  assign n8673 = ~pmp_fault_i;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:138:51  */
  assign n8674 = n8672 & n8673;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:140:21  */
  assign n8675 = req[36:5]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:147:16  */
  assign n8677 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:151:18  */
  assign n8679 = n8554[161]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:152:30  */
  assign n8680 = n8554[221:220]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:154:26  */
  assign n8681 = req[6:5]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:155:74  */
  assign n8683 = n8554[222]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:155:54  */
  assign n8684 = ~n8683;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:155:98  */
  assign n8685 = n8570[9]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:155:79  */
  assign n8686 = n8684 & n8685;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8692 = {n8686, n8686, n8686, n8686};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8693 = {n8686, n8686, n8686, n8686};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8694 = {n8686, n8686, n8686, n8686};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8695 = {n8686, n8686, n8686, n8686};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8696 = {n8686, n8686, n8686, n8686};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8697 = {n8686, n8686, n8686, n8686};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8698 = {n8692, n8693, n8694, n8695};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:29  */
  assign n8699 = {n8696, n8697};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8700 = {n8698, n8699};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:155:125  */
  assign n8702 = n8570[9:2]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:155:108  */
  assign n8703 = {n8700, n8702};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:155:15  */
  assign n8705 = n8681 == 2'b00;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:156:74  */
  assign n8707 = n8554[222]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:156:54  */
  assign n8708 = ~n8707;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:156:98  */
  assign n8709 = n8570[17]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:156:79  */
  assign n8710 = n8708 & n8709;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8716 = {n8710, n8710, n8710, n8710};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8717 = {n8710, n8710, n8710, n8710};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8718 = {n8710, n8710, n8710, n8710};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8719 = {n8710, n8710, n8710, n8710};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8720 = {n8710, n8710, n8710, n8710};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8721 = {n8710, n8710, n8710, n8710};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8722 = {n8716, n8717, n8718, n8719};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:29  */
  assign n8723 = {n8720, n8721};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8724 = {n8722, n8723};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:156:125  */
  assign n8726 = n8570[17:10]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:156:108  */
  assign n8727 = {n8724, n8726};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:156:15  */
  assign n8729 = n8681 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:157:74  */
  assign n8731 = n8554[222]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:157:54  */
  assign n8732 = ~n8731;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:157:98  */
  assign n8733 = n8570[25]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:157:79  */
  assign n8734 = n8732 & n8733;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8740 = {n8734, n8734, n8734, n8734};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8741 = {n8734, n8734, n8734, n8734};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8742 = {n8734, n8734, n8734, n8734};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8743 = {n8734, n8734, n8734, n8734};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8744 = {n8734, n8734, n8734, n8734};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8745 = {n8734, n8734, n8734, n8734};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8746 = {n8740, n8741, n8742, n8743};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:29  */
  assign n8747 = {n8744, n8745};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8748 = {n8746, n8747};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:157:125  */
  assign n8750 = n8570[25:18]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:157:108  */
  assign n8751 = {n8748, n8750};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:157:15  */
  assign n8753 = n8681 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:158:74  */
  assign n8755 = n8554[222]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:158:54  */
  assign n8756 = ~n8755;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:158:98  */
  assign n8757 = n8570[33]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:158:79  */
  assign n8758 = n8756 & n8757;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8764 = {n8758, n8758, n8758, n8758};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8765 = {n8758, n8758, n8758, n8758};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8766 = {n8758, n8758, n8758, n8758};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8767 = {n8758, n8758, n8758, n8758};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8768 = {n8758, n8758, n8758, n8758};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8769 = {n8758, n8758, n8758, n8758};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8770 = {n8764, n8765, n8766, n8767};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:29  */
  assign n8771 = {n8768, n8769};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8772 = {n8770, n8771};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:158:125  */
  assign n8774 = n8570[33:26]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:158:108  */
  assign n8775 = {n8772, n8774};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8776 = {n8753, n8729, n8705};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:154:13  */
  always @*
    case (n8776)
      3'b100: n8777 = n8751;
      3'b010: n8777 = n8727;
      3'b001: n8777 = n8703;
      default: n8777 = n8775;
    endcase
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:153:11  */
  assign n8779 = n8680 == 2'b00;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:161:25  */
  assign n8780 = req[6]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:161:29  */
  assign n8781 = ~n8780;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:162:59  */
  assign n8783 = n8554[222]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:162:39  */
  assign n8784 = ~n8783;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:162:83  */
  assign n8785 = n8570[17]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:162:64  */
  assign n8786 = n8784 & n8785;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:49:19  */
  assign n8792 = {n8786, n8786, n8786, n8786};
  assign n8793 = {n8786, n8786, n8786, n8786};
  assign n8794 = {n8786, n8786, n8786, n8786};
  assign n8795 = {n8786, n8786, n8786, n8786};
  assign n8796 = {n8792, n8793, n8794, n8795};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:162:110  */
  assign n8798 = n8570[17:2]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:162:93  */
  assign n8799 = {n8796, n8798};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:164:59  */
  assign n8801 = n8554[222]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:164:39  */
  assign n8802 = ~n8801;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:164:83  */
  assign n8803 = n8570[33]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:164:64  */
  assign n8804 = n8802 & n8803;
  assign n8810 = {n8804, n8804, n8804, n8804};
  assign n8811 = {n8804, n8804, n8804, n8804};
  assign n8812 = {n8804, n8804, n8804, n8804};
  assign n8813 = {n8804, n8804, n8804, n8804};
  assign n8814 = {n8810, n8811, n8812, n8813};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:164:110  */
  assign n8816 = n8570[33:18]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:164:93  */
  assign n8817 = {n8814, n8816};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:161:13  */
  assign n8818 = n8781 ? n8799 : n8817;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:160:11  */
  assign n8820 = n8680 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:167:35  */
  assign n8821 = n8570[33:2]; // extract
  assign n8822 = {n8820, n8779};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:152:9  */
  always @*
    case (n8822)
      2'b10: n8823 = n8818;
      2'b01: n8823 = n8777;
      default: n8823 = n8821;
    endcase
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:151:7  */
  assign n8825 = n8679 ? n8823 : 32'b00000000000000000000000000000000;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:174:28  */
  assign n8831 = n8570[0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:174:13  */
  assign n8832 = ~n8831;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:179:22  */
  assign n8833 = n8554[161]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:179:43  */
  assign n8834 = n8554[158]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:179:32  */
  assign n8835 = n8833 & n8834;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:179:50  */
  assign n8836 = n8835 & misalign;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:180:22  */
  assign n8837 = n8554[161]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:180:43  */
  assign n8838 = n8554[158]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:180:32  */
  assign n8839 = n8837 & n8838;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:180:66  */
  assign n8840 = n8570[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:180:70  */
  assign n8841 = n8840 | pmp_fault_i;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:180:50  */
  assign n8842 = n8839 & n8841;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:181:22  */
  assign n8843 = n8554[161]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:181:43  */
  assign n8844 = n8554[159]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:181:32  */
  assign n8845 = n8843 & n8844;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:181:50  */
  assign n8846 = n8845 & misalign;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:182:22  */
  assign n8847 = n8554[161]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:182:43  */
  assign n8848 = n8554[159]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:182:32  */
  assign n8849 = n8847 & n8848;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:182:66  */
  assign n8850 = n8570[1]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:182:70  */
  assign n8851 = n8850 | pmp_fault_i;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:182:50  */
  assign n8852 = n8849 & n8851;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:107:5  */
  assign n8853 = req[74]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:107:5  */
  assign n8854 = n8582 ? n8651 : n8853;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:107:5  */
  always @(posedge clk_i or posedge n8575)
    if (n8575)
      n8855 <= 1'b0;
    else
      n8855 <= n8854;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:107:5  */
  assign n8856 = req[72:0]; // extract
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:107:5  */
  assign n8857 = n8582 ? n8652 : n8856;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:107:5  */
  always @(posedge clk_i or posedge n8575)
    if (n8575)
      n8858 <= n8663;
    else
      n8858 <= n8857;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:100:5  */
  assign n8859 = {1'b0, 1'b0, 4'b0000, 1'b0, n8855, n8674, n8858};
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:107:5  */
  assign n8860 = n8582 ? n8650 : misalign;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:107:5  */
  always @(posedge clk_i or posedge n8575)
    if (n8575)
      n8861 <= 1'b0;
    else
      n8861 <= n8860;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:149:5  */
  always @(posedge clk_i or posedge n8677)
    if (n8677)
      n8862 <= 32'b00000000000000000000000000000000;
    else
      n8862 <= n8825;
  /* ../../rtl/core/neorv32_cpu_lsu.vhd:147:5  */
  assign n8863 = {n8852, n8846, n8842, n8836};
endmodule

module neorv32_cpu_alu_9a68e0f891a604eadc414df454e914fb8b2693a9
  (input  clk_i,
   input  rstn_i,
   input  \ctrl_i_ctrl_i[if_reset] ,
   input  \ctrl_i_ctrl_i[if_ready] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_cur] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_nxt] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_ret] ,
   input  \ctrl_i_ctrl_i[rf_wb_en] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs1] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs2] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rd] ,
   input  \ctrl_i_ctrl_i[rf_zero] ,
   input  [2:0] \ctrl_i_ctrl_i[alu_op] ,
   input  \ctrl_i_ctrl_i[alu_sub] ,
   input  \ctrl_i_ctrl_i[alu_opa_mux] ,
   input  \ctrl_i_ctrl_i[alu_opb_mux] ,
   input  \ctrl_i_ctrl_i[alu_unsigned] ,
   input  [31:0] \ctrl_i_ctrl_i[alu_imm] ,
   input  \ctrl_i_ctrl_i[alu_cp_alu] ,
   input  \ctrl_i_ctrl_i[alu_cp_cfu] ,
   input  \ctrl_i_ctrl_i[alu_cp_fpu] ,
   input  \ctrl_i_ctrl_i[lsu_req] ,
   input  \ctrl_i_ctrl_i[lsu_rd] ,
   input  \ctrl_i_ctrl_i[lsu_wr] ,
   input  \ctrl_i_ctrl_i[lsu_mo_en] ,
   input  \ctrl_i_ctrl_i[lsu_mi_en] ,
   input  \ctrl_i_ctrl_i[lsu_priv] ,
   input  \ctrl_i_ctrl_i[csr_we] ,
   input  \ctrl_i_ctrl_i[csr_re] ,
   input  [11:0] \ctrl_i_ctrl_i[csr_addr] ,
   input  [31:0] \ctrl_i_ctrl_i[csr_wdata] ,
   input  [10:0] \ctrl_i_ctrl_i[cnt_event] ,
   input  [2:0] \ctrl_i_ctrl_i[ir_funct3] ,
   input  [11:0] \ctrl_i_ctrl_i[ir_funct12] ,
   input  [6:0] \ctrl_i_ctrl_i[ir_opcode] ,
   input  [15:0] \ctrl_i_ctrl_i[ir_rvc] ,
   input  \ctrl_i_ctrl_i[cpu_priv] ,
   input  \ctrl_i_ctrl_i[cpu_trap] ,
   input  \ctrl_i_ctrl_i[cpu_sync_exc] ,
   input  \ctrl_i_ctrl_i[cpu_debug] ,
   input  [1:0] \ctrl_i_ctrl_i[cpu_fence] ,
   input  [31:0] rs1_i,
   input  [31:0] rs2_i,
   output [1:0] cmp_o,
   output [31:0] res_o,
   output [31:0] add_o,
   output [31:0] csr_o,
   output done_o);
  wire [263:0] n8332;
  wire [31:0] opa;
  wire [31:0] opb;
  wire [31:0] cp_res;
  wire [32:0] cmp_rs1;
  wire [32:0] cmp_rs2;
  wire [32:0] opa_x;
  wire [32:0] opb_x;
  wire [32:0] addsub;
  wire [1:0] cmp;
  wire [223:0] cp_result;
  wire [6:0] cp_valid;
  wire n8338;
  wire n8339;
  wire n8340;
  wire n8341;
  wire [32:0] n8342;
  wire n8343;
  wire n8344;
  wire n8345;
  wire n8346;
  wire [32:0] n8347;
  wire n8349;
  wire n8350;
  wire n8353;
  wire n8354;
  wire [2:0] n8357;
  wire n8359;
  wire [31:0] n8360;
  wire n8362;
  wire n8364;
  wire n8365;
  wire n8367;
  wire n8369;
  wire [31:0] n8370;
  wire n8372;
  wire [31:0] n8373;
  wire n8375;
  wire [31:0] n8376;
  wire n8378;
  wire [7:0] n8379;
  wire n8381;
  wire n8382;
  wire n8383;
  wire n8384;
  wire n8385;
  wire n8386;
  reg n8388;
  wire [30:0] n8390;
  wire [30:0] n8391;
  wire [30:0] n8392;
  wire [30:0] n8393;
  wire [30:0] n8394;
  wire [30:0] n8395;
  reg [30:0] n8398;
  wire [31:0] n8402;
  wire n8403;
  wire [31:0] n8404;
  wire [31:0] n8405;
  wire n8406;
  wire [31:0] n8407;
  wire n8408;
  wire n8409;
  wire n8410;
  wire n8411;
  wire [32:0] n8412;
  wire n8413;
  wire n8414;
  wire n8415;
  wire n8416;
  wire [32:0] n8417;
  wire [31:0] n8418;
  wire [32:0] n8419;
  wire n8420;
  wire [32:0] n8421;
  wire [32:0] n8422;
  wire n8423;
  wire n8424;
  wire n8425;
  wire n8426;
  wire n8427;
  wire n8428;
  wire n8429;
  wire n8430;
  wire n8431;
  wire n8432;
  wire n8433;
  wire n8434;
  wire n8435;
  wire [31:0] n8436;
  wire [31:0] n8437;
  wire [31:0] n8438;
  wire [31:0] n8439;
  wire [31:0] n8440;
  wire [31:0] n8441;
  wire [31:0] n8442;
  wire [31:0] n8443;
  wire [31:0] n8444;
  wire [31:0] n8445;
  wire [31:0] n8446;
  wire [31:0] n8447;
  wire [31:0] n8448;
  wire [31:0] \neorv32_cpu_alu_shifter_inst.res_o ;
  wire \neorv32_cpu_alu_shifter_inst.valid_o ;
  wire n8449;
  wire n8450;
  wire [31:0] n8451;
  wire [31:0] n8452;
  wire [31:0] n8453;
  wire n8454;
  wire [4:0] n8455;
  wire [4:0] n8456;
  wire [4:0] n8457;
  wire n8458;
  wire [2:0] n8459;
  wire n8460;
  wire n8461;
  wire n8462;
  wire n8463;
  wire [31:0] n8464;
  wire n8465;
  wire n8466;
  wire n8467;
  wire n8468;
  wire n8469;
  wire n8470;
  wire n8471;
  wire n8472;
  wire n8473;
  wire n8474;
  wire n8475;
  wire [11:0] n8476;
  wire [31:0] n8477;
  wire [10:0] n8478;
  wire [2:0] n8479;
  wire [11:0] n8480;
  wire [6:0] n8481;
  wire [15:0] n8482;
  wire n8483;
  wire n8484;
  wire n8485;
  wire n8486;
  wire [1:0] n8487;
  wire [4:0] n8488;
  wire [31:0] \neorv32_cpu_alu_muldiv_enabled_neorv32_cpu_alu_muldiv_inst.res_o ;
  wire \neorv32_cpu_alu_muldiv_enabled_neorv32_cpu_alu_muldiv_inst.valid_o ;
  wire n8491;
  wire n8492;
  wire [31:0] n8493;
  wire [31:0] n8494;
  wire [31:0] n8495;
  wire n8496;
  wire [4:0] n8497;
  wire [4:0] n8498;
  wire [4:0] n8499;
  wire n8500;
  wire [2:0] n8501;
  wire n8502;
  wire n8503;
  wire n8504;
  wire n8505;
  wire [31:0] n8506;
  wire n8507;
  wire n8508;
  wire n8509;
  wire n8510;
  wire n8511;
  wire n8512;
  wire n8513;
  wire n8514;
  wire n8515;
  wire n8516;
  wire n8517;
  wire [11:0] n8518;
  wire [31:0] n8519;
  wire [10:0] n8520;
  wire [2:0] n8521;
  wire [11:0] n8522;
  wire [6:0] n8523;
  wire [15:0] n8524;
  wire n8525;
  wire n8526;
  wire n8527;
  wire n8528;
  wire [1:0] n8529;
  localparam [31:0] n8537 = 32'b00000000000000000000000000000000;
  wire [1:0] n8550;
  wire [223:0] n8551;
  wire [6:0] n8552;
  wire [31:0] n8553;
  assign cmp_o = cmp; //(module output)
  assign res_o = n8553; //(module output)
  assign add_o = n8418; //(module output)
  assign csr_o = n8537; //(module output)
  assign done_o = n8435; //(module output)
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:74:7  */
  assign n8332 = {\ctrl_i_ctrl_i[cpu_fence] , \ctrl_i_ctrl_i[cpu_debug] , \ctrl_i_ctrl_i[cpu_sync_exc] , \ctrl_i_ctrl_i[cpu_trap] , \ctrl_i_ctrl_i[cpu_priv] , \ctrl_i_ctrl_i[ir_rvc] , \ctrl_i_ctrl_i[ir_opcode] , \ctrl_i_ctrl_i[ir_funct12] , \ctrl_i_ctrl_i[ir_funct3] , \ctrl_i_ctrl_i[cnt_event] , \ctrl_i_ctrl_i[csr_wdata] , \ctrl_i_ctrl_i[csr_addr] , \ctrl_i_ctrl_i[csr_re] , \ctrl_i_ctrl_i[csr_we] , \ctrl_i_ctrl_i[lsu_priv] , \ctrl_i_ctrl_i[lsu_mi_en] , \ctrl_i_ctrl_i[lsu_mo_en] , \ctrl_i_ctrl_i[lsu_wr] , \ctrl_i_ctrl_i[lsu_rd] , \ctrl_i_ctrl_i[lsu_req] , \ctrl_i_ctrl_i[alu_cp_fpu] , \ctrl_i_ctrl_i[alu_cp_cfu] , \ctrl_i_ctrl_i[alu_cp_alu] , \ctrl_i_ctrl_i[alu_imm] , \ctrl_i_ctrl_i[alu_unsigned] , \ctrl_i_ctrl_i[alu_opb_mux] , \ctrl_i_ctrl_i[alu_opa_mux] , \ctrl_i_ctrl_i[alu_sub] , \ctrl_i_ctrl_i[alu_op] , \ctrl_i_ctrl_i[rf_zero] , \ctrl_i_ctrl_i[rf_rd] , \ctrl_i_ctrl_i[rf_rs2] , \ctrl_i_ctrl_i[rf_rs1] , \ctrl_i_ctrl_i[rf_wb_en] , \ctrl_i_ctrl_i[pc_ret] , \ctrl_i_ctrl_i[pc_nxt] , \ctrl_i_ctrl_i[pc_cur] , \ctrl_i_ctrl_i[if_ready] , \ctrl_i_ctrl_i[if_reset] };
  /* ../../rtl/core/neorv32_cpu_alu.vhd:76:10  */
  assign opa = n8404; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:76:15  */
  assign opb = n8407; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:76:20  */
  assign cp_res = n8448; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:77:10  */
  assign cmp_rs1 = n8342; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:77:19  */
  assign cmp_rs2 = n8347; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:77:28  */
  assign opa_x = n8412; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:77:35  */
  assign opb_x = n8417; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:77:42  */
  assign addsub = n8421; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:78:10  */
  assign cmp = n8550; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:82:10  */
  assign cp_result = n8551; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:83:10  */
  assign cp_valid = n8552; // (signal)
  /* ../../rtl/core/neorv32_cpu_alu.vhd:92:20  */
  assign n8338 = rs1_i[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:92:49  */
  assign n8339 = n8332[121]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:92:38  */
  assign n8340 = ~n8339;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:92:33  */
  assign n8341 = n8338 & n8340;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:92:64  */
  assign n8342 = {n8341, rs1_i};
  /* ../../rtl/core/neorv32_cpu_alu.vhd:93:20  */
  assign n8343 = rs2_i[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:93:49  */
  assign n8344 = n8332[121]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:93:38  */
  assign n8345 = ~n8344;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:93:33  */
  assign n8346 = n8343 & n8345;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:93:64  */
  assign n8347 = {n8346, rs2_i};
  /* ../../rtl/core/neorv32_cpu_alu.vhd:94:29  */
  assign n8349 = rs1_i == rs2_i;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:94:17  */
  assign n8350 = n8349 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:95:39  */
  assign n8353 = $signed(cmp_rs1) < $signed(cmp_rs2);
  /* ../../rtl/core/neorv32_cpu_alu.vhd:95:17  */
  assign n8354 = n8353 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:113:17  */
  assign n8357 = n8332[117:115]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:114:7  */
  assign n8359 = n8357 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:115:44  */
  assign n8360 = addsub[31:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:115:7  */
  assign n8362 = n8357 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:116:7  */
  assign n8364 = n8357 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:117:47  */
  assign n8365 = addsub[32]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:117:7  */
  assign n8367 = n8357 == 3'b011;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:118:7  */
  assign n8369 = n8357 == 3'b100;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:119:42  */
  assign n8370 = opb ^ rs1_i;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:119:7  */
  assign n8372 = n8357 == 3'b101;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:120:42  */
  assign n8373 = opb | rs1_i;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:120:7  */
  assign n8375 = n8357 == 3'b110;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:121:42  */
  assign n8376 = opb & rs1_i;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:121:7  */
  assign n8378 = n8357 == 3'b111;
  assign n8379 = {n8378, n8375, n8372, n8369, n8367, n8364, n8362, n8359};
  assign n8381 = n8360[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8382 = cp_res[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8383 = opb[0]; // extract
  assign n8384 = n8370[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8385 = n8373[0]; // extract
  assign n8386 = n8376[0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:113:5  */
  always @*
    case (n8379)
      8'b10000000: n8388 = n8386;
      8'b01000000: n8388 = n8385;
      8'b00100000: n8388 = n8384;
      8'b00010000: n8388 = n8383;
      8'b00001000: n8388 = n8365;
      8'b00000100: n8388 = n8382;
      8'b00000010: n8388 = n8381;
      8'b00000001: n8388 = 1'b0;
      default: n8388 = 1'b0;
    endcase
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8390 = n8360[31:1]; // extract
  assign n8391 = cp_res[31:1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8392 = opb[31:1]; // extract
  assign n8393 = n8370[31:1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1216:14  */
  assign n8394 = n8373[31:1]; // extract
  assign n8395 = n8376[31:1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:113:5  */
  always @*
    case (n8379)
      8'b10000000: n8398 = n8395;
      8'b01000000: n8398 = n8394;
      8'b00100000: n8398 = n8393;
      8'b00010000: n8398 = n8392;
      8'b00001000: n8398 = 31'b0000000000000000000000000000000;
      8'b00000100: n8398 = n8391;
      8'b00000010: n8398 = n8390;
      8'b00000001: n8398 = 31'b0000000000000000000000000000000;
      default: n8398 = 31'b0000000000000000000000000000000;
    endcase
  /* ../../rtl/core/neorv32_cpu_alu.vhd:127:19  */
  assign n8402 = n8332[33:2]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:127:40  */
  assign n8403 = n8332[119]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:127:27  */
  assign n8404 = n8403 ? n8402 : rs1_i;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:128:19  */
  assign n8405 = n8332[153:122]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:128:40  */
  assign n8406 = n8332[120]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:128:27  */
  assign n8407 = n8406 ? n8405 : rs2_i;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:129:16  */
  assign n8408 = opa[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:129:43  */
  assign n8409 = n8332[121]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:129:32  */
  assign n8410 = ~n8409;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:129:27  */
  assign n8411 = n8408 & n8410;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:129:58  */
  assign n8412 = {n8411, opa};
  /* ../../rtl/core/neorv32_cpu_alu.vhd:130:16  */
  assign n8413 = opb[31]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:130:43  */
  assign n8414 = n8332[121]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:130:32  */
  assign n8415 = ~n8414;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:130:27  */
  assign n8416 = n8413 & n8415;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:130:58  */
  assign n8417 = {n8416, opb};
  /* ../../rtl/core/neorv32_cpu_alu.vhd:133:19  */
  assign n8418 = addsub[31:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:134:47  */
  assign n8419 = opa_x - opb_x;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:134:79  */
  assign n8420 = n8332[118]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:134:66  */
  assign n8421 = n8420 ? n8419 : n8422;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:135:47  */
  assign n8422 = opa_x + opb_x;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:21  */
  assign n8423 = cp_valid[0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:36  */
  assign n8424 = cp_valid[1]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:25  */
  assign n8425 = n8423 | n8424;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:51  */
  assign n8426 = cp_valid[2]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:40  */
  assign n8427 = n8425 | n8426;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:66  */
  assign n8428 = cp_valid[3]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:55  */
  assign n8429 = n8427 | n8428;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:81  */
  assign n8430 = cp_valid[4]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:70  */
  assign n8431 = n8429 | n8430;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:96  */
  assign n8432 = cp_valid[5]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:85  */
  assign n8433 = n8431 | n8432;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:111  */
  assign n8434 = cp_valid[6]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:142:100  */
  assign n8435 = n8433 | n8434;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:22  */
  assign n8436 = cp_result[223:192]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:38  */
  assign n8437 = cp_result[191:160]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:26  */
  assign n8438 = n8436 | n8437;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:54  */
  assign n8439 = cp_result[159:128]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:42  */
  assign n8440 = n8438 | n8439;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:70  */
  assign n8441 = cp_result[127:96]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:58  */
  assign n8442 = n8440 | n8441;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:86  */
  assign n8443 = cp_result[95:64]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:74  */
  assign n8444 = n8442 | n8443;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:102  */
  assign n8445 = cp_result[63:32]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:90  */
  assign n8446 = n8444 | n8445;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:118  */
  assign n8447 = cp_result[31:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:143:106  */
  assign n8448 = n8446 | n8447;
  /* ../../rtl/core/neorv32_cpu_alu.vhd:147:3  */
  neorv32_cpu_alu_shifter_5ba93c9db0cff93f52b521d7420e43f6eda2784f neorv32_cpu_alu_shifter_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\ctrl_i_ctrl_i[if_reset] (n8449),
    .\ctrl_i_ctrl_i[if_ready] (n8450),
    .\ctrl_i_ctrl_i[pc_cur] (n8451),
    .\ctrl_i_ctrl_i[pc_nxt] (n8452),
    .\ctrl_i_ctrl_i[pc_ret] (n8453),
    .\ctrl_i_ctrl_i[rf_wb_en] (n8454),
    .\ctrl_i_ctrl_i[rf_rs1] (n8455),
    .\ctrl_i_ctrl_i[rf_rs2] (n8456),
    .\ctrl_i_ctrl_i[rf_rd] (n8457),
    .\ctrl_i_ctrl_i[rf_zero] (n8458),
    .\ctrl_i_ctrl_i[alu_op] (n8459),
    .\ctrl_i_ctrl_i[alu_sub] (n8460),
    .\ctrl_i_ctrl_i[alu_opa_mux] (n8461),
    .\ctrl_i_ctrl_i[alu_opb_mux] (n8462),
    .\ctrl_i_ctrl_i[alu_unsigned] (n8463),
    .\ctrl_i_ctrl_i[alu_imm] (n8464),
    .\ctrl_i_ctrl_i[alu_cp_alu] (n8465),
    .\ctrl_i_ctrl_i[alu_cp_cfu] (n8466),
    .\ctrl_i_ctrl_i[alu_cp_fpu] (n8467),
    .\ctrl_i_ctrl_i[lsu_req] (n8468),
    .\ctrl_i_ctrl_i[lsu_rd] (n8469),
    .\ctrl_i_ctrl_i[lsu_wr] (n8470),
    .\ctrl_i_ctrl_i[lsu_mo_en] (n8471),
    .\ctrl_i_ctrl_i[lsu_mi_en] (n8472),
    .\ctrl_i_ctrl_i[lsu_priv] (n8473),
    .\ctrl_i_ctrl_i[csr_we] (n8474),
    .\ctrl_i_ctrl_i[csr_re] (n8475),
    .\ctrl_i_ctrl_i[csr_addr] (n8476),
    .\ctrl_i_ctrl_i[csr_wdata] (n8477),
    .\ctrl_i_ctrl_i[cnt_event] (n8478),
    .\ctrl_i_ctrl_i[ir_funct3] (n8479),
    .\ctrl_i_ctrl_i[ir_funct12] (n8480),
    .\ctrl_i_ctrl_i[ir_opcode] (n8481),
    .\ctrl_i_ctrl_i[ir_rvc] (n8482),
    .\ctrl_i_ctrl_i[cpu_priv] (n8483),
    .\ctrl_i_ctrl_i[cpu_trap] (n8484),
    .\ctrl_i_ctrl_i[cpu_sync_exc] (n8485),
    .\ctrl_i_ctrl_i[cpu_debug] (n8486),
    .\ctrl_i_ctrl_i[cpu_fence] (n8487),
    .rs1_i(rs1_i),
    .shamt_i(n8488),
    .res_o(\neorv32_cpu_alu_shifter_inst.res_o ),
    .valid_o(\neorv32_cpu_alu_shifter_inst.valid_o ));
  /* ../../rtl/core/neorv32_package.vhd:1216:14  */
  assign n8449 = n8332[0]; // extract
  assign n8450 = n8332[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8451 = n8332[33:2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8452 = n8332[65:34]; // extract
  assign n8453 = n8332[97:66]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8454 = n8332[98]; // extract
  assign n8455 = n8332[103:99]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1216:14  */
  assign n8456 = n8332[108:104]; // extract
  assign n8457 = n8332[113:109]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8458 = n8332[114]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8459 = n8332[117:115]; // extract
  assign n8460 = n8332[118]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8461 = n8332[119]; // extract
  assign n8462 = n8332[120]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1216:14  */
  assign n8463 = n8332[121]; // extract
  assign n8464 = n8332[153:122]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8465 = n8332[154]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8466 = n8332[155]; // extract
  assign n8467 = n8332[156]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8468 = n8332[157]; // extract
  assign n8469 = n8332[158]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1216:14  */
  assign n8470 = n8332[159]; // extract
  assign n8471 = n8332[160]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8472 = n8332[161]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8473 = n8332[162]; // extract
  assign n8474 = n8332[163]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8475 = n8332[164]; // extract
  assign n8476 = n8332[176:165]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1216:14  */
  assign n8477 = n8332[208:177]; // extract
  assign n8478 = n8332[219:209]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8479 = n8332[222:220]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8480 = n8332[234:223]; // extract
  assign n8481 = n8332[241:235]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8482 = n8332[257:242]; // extract
  assign n8483 = n8332[258]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1216:14  */
  assign n8484 = n8332[259]; // extract
  assign n8485 = n8332[260]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8486 = n8332[261]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n8487 = n8332[263:262]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:158:19  */
  assign n8488 = opb[4:0]; // extract
  /* ../../rtl/core/neorv32_cpu_alu.vhd:168:5  */
  neorv32_cpu_alu_muldiv_3f29546453678b855931c174a97d6c0894b8f546 neorv32_cpu_alu_muldiv_enabled_neorv32_cpu_alu_muldiv_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\ctrl_i_ctrl_i[if_reset] (n8491),
    .\ctrl_i_ctrl_i[if_ready] (n8492),
    .\ctrl_i_ctrl_i[pc_cur] (n8493),
    .\ctrl_i_ctrl_i[pc_nxt] (n8494),
    .\ctrl_i_ctrl_i[pc_ret] (n8495),
    .\ctrl_i_ctrl_i[rf_wb_en] (n8496),
    .\ctrl_i_ctrl_i[rf_rs1] (n8497),
    .\ctrl_i_ctrl_i[rf_rs2] (n8498),
    .\ctrl_i_ctrl_i[rf_rd] (n8499),
    .\ctrl_i_ctrl_i[rf_zero] (n8500),
    .\ctrl_i_ctrl_i[alu_op] (n8501),
    .\ctrl_i_ctrl_i[alu_sub] (n8502),
    .\ctrl_i_ctrl_i[alu_opa_mux] (n8503),
    .\ctrl_i_ctrl_i[alu_opb_mux] (n8504),
    .\ctrl_i_ctrl_i[alu_unsigned] (n8505),
    .\ctrl_i_ctrl_i[alu_imm] (n8506),
    .\ctrl_i_ctrl_i[alu_cp_alu] (n8507),
    .\ctrl_i_ctrl_i[alu_cp_cfu] (n8508),
    .\ctrl_i_ctrl_i[alu_cp_fpu] (n8509),
    .\ctrl_i_ctrl_i[lsu_req] (n8510),
    .\ctrl_i_ctrl_i[lsu_rd] (n8511),
    .\ctrl_i_ctrl_i[lsu_wr] (n8512),
    .\ctrl_i_ctrl_i[lsu_mo_en] (n8513),
    .\ctrl_i_ctrl_i[lsu_mi_en] (n8514),
    .\ctrl_i_ctrl_i[lsu_priv] (n8515),
    .\ctrl_i_ctrl_i[csr_we] (n8516),
    .\ctrl_i_ctrl_i[csr_re] (n8517),
    .\ctrl_i_ctrl_i[csr_addr] (n8518),
    .\ctrl_i_ctrl_i[csr_wdata] (n8519),
    .\ctrl_i_ctrl_i[cnt_event] (n8520),
    .\ctrl_i_ctrl_i[ir_funct3] (n8521),
    .\ctrl_i_ctrl_i[ir_funct12] (n8522),
    .\ctrl_i_ctrl_i[ir_opcode] (n8523),
    .\ctrl_i_ctrl_i[ir_rvc] (n8524),
    .\ctrl_i_ctrl_i[cpu_priv] (n8525),
    .\ctrl_i_ctrl_i[cpu_trap] (n8526),
    .\ctrl_i_ctrl_i[cpu_sync_exc] (n8527),
    .\ctrl_i_ctrl_i[cpu_debug] (n8528),
    .\ctrl_i_ctrl_i[cpu_fence] (n8529),
    .rs1_i(rs1_i),
    .rs2_i(rs2_i),
    .res_o(\neorv32_cpu_alu_muldiv_enabled_neorv32_cpu_alu_muldiv_inst.res_o ),
    .valid_o(\neorv32_cpu_alu_muldiv_enabled_neorv32_cpu_alu_muldiv_inst.valid_o ));
  assign n8491 = n8332[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:95:3  */
  assign n8492 = n8332[1]; // extract
  assign n8493 = n8332[33:2]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:86:18  */
  assign n8494 = n8332[65:34]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:86:53  */
  assign n8495 = n8332[97:66]; // extract
  assign n8496 = n8332[98]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:86:39  */
  assign n8497 = n8332[103:99]; // extract
  assign n8498 = n8332[108:104]; // extract
  assign n8499 = n8332[113:109]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:84:18  */
  assign n8500 = n8332[114]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:84:53  */
  assign n8501 = n8332[117:115]; // extract
  assign n8502 = n8332[118]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:84:39  */
  assign n8503 = n8332[119]; // extract
  assign n8504 = n8332[120]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8505 = n8332[121]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8506 = n8332[153:122]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:29  */
  assign n8507 = n8332[154]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8508 = n8332[155]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8509 = n8332[156]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8510 = n8332[157]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8511 = n8332[158]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8512 = n8332[159]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8513 = n8332[160]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8514 = n8332[161]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:29  */
  assign n8515 = n8332[162]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8516 = n8332[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8517 = n8332[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8518 = n8332[176:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8519 = n8332[208:177]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8520 = n8332[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8521 = n8332[222:220]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:51  */
  assign n8522 = n8332[234:223]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:29  */
  assign n8523 = n8332[241:235]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:36  */
  assign n8524 = n8332[257:242]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:40  */
  assign n8525 = n8332[258]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8526 = n8332[259]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8527 = n8332[260]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8528 = n8332[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8529 = n8332[263:262]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:51  */
  assign n8550 = {n8354, n8350};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:29  */
  assign n8551 = {\neorv32_cpu_alu_shifter_inst.res_o , \neorv32_cpu_alu_muldiv_enabled_neorv32_cpu_alu_muldiv_inst.res_o , 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:75:36  */
  assign n8552 = {1'b0, 1'b0, 1'b0, 1'b0, 1'b0, \neorv32_cpu_alu_muldiv_enabled_neorv32_cpu_alu_muldiv_inst.valid_o , \neorv32_cpu_alu_shifter_inst.valid_o };
  /* ../../rtl/core/neorv32_cpu_counters.vhd:76:40  */
  assign n8553 = {n8398, n8388};
endmodule

module neorv32_cpu_regfile_32_5_0
  (input  clk_i,
   input  rstn_i,
   input  \ctrl_i_ctrl_i[if_reset] ,
   input  \ctrl_i_ctrl_i[if_ready] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_cur] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_nxt] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_ret] ,
   input  \ctrl_i_ctrl_i[rf_wb_en] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs1] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs2] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rd] ,
   input  \ctrl_i_ctrl_i[rf_zero] ,
   input  [2:0] \ctrl_i_ctrl_i[alu_op] ,
   input  \ctrl_i_ctrl_i[alu_sub] ,
   input  \ctrl_i_ctrl_i[alu_opa_mux] ,
   input  \ctrl_i_ctrl_i[alu_opb_mux] ,
   input  \ctrl_i_ctrl_i[alu_unsigned] ,
   input  [31:0] \ctrl_i_ctrl_i[alu_imm] ,
   input  \ctrl_i_ctrl_i[alu_cp_alu] ,
   input  \ctrl_i_ctrl_i[alu_cp_cfu] ,
   input  \ctrl_i_ctrl_i[alu_cp_fpu] ,
   input  \ctrl_i_ctrl_i[lsu_req] ,
   input  \ctrl_i_ctrl_i[lsu_rd] ,
   input  \ctrl_i_ctrl_i[lsu_wr] ,
   input  \ctrl_i_ctrl_i[lsu_mo_en] ,
   input  \ctrl_i_ctrl_i[lsu_mi_en] ,
   input  \ctrl_i_ctrl_i[lsu_priv] ,
   input  \ctrl_i_ctrl_i[csr_we] ,
   input  \ctrl_i_ctrl_i[csr_re] ,
   input  [11:0] \ctrl_i_ctrl_i[csr_addr] ,
   input  [31:0] \ctrl_i_ctrl_i[csr_wdata] ,
   input  [10:0] \ctrl_i_ctrl_i[cnt_event] ,
   input  [2:0] \ctrl_i_ctrl_i[ir_funct3] ,
   input  [11:0] \ctrl_i_ctrl_i[ir_funct12] ,
   input  [6:0] \ctrl_i_ctrl_i[ir_opcode] ,
   input  [15:0] \ctrl_i_ctrl_i[ir_rvc] ,
   input  \ctrl_i_ctrl_i[cpu_priv] ,
   input  \ctrl_i_ctrl_i[cpu_trap] ,
   input  \ctrl_i_ctrl_i[cpu_sync_exc] ,
   input  \ctrl_i_ctrl_i[cpu_debug] ,
   input  [1:0] \ctrl_i_ctrl_i[cpu_fence] ,
   input  [31:0] rd_i,
   output [31:0] rs1_o,
   output [31:0] rs2_o);
  wire [263:0] n8271;
  wire rf_we;
  wire [4:0] addr;
  wire n8274;
  wire n8282;
  wire n8284;
  wire n8286;
  wire n8287;
  wire n8288;
  wire n8289;
  wire n8290;
  wire n8291;
  wire n8292;
  wire n8293;
  wire n8294;
  wire n8295;
  wire n8296;
  wire n8298;
  wire [4:0] n8299;
  wire [4:0] n8300;
  wire n8301;
  wire [4:0] n8302;
  wire [4:0] n8303;
  wire [4:0] n8313;
  reg [31:0] n8328; // mem_rd
  reg [31:0] n8330; // mem_rd
  assign rs1_o = n8330; //(module output)
  assign rs2_o = n8328; //(module output)
  assign n8271 = {\ctrl_i_ctrl_i[cpu_fence] , \ctrl_i_ctrl_i[cpu_debug] , \ctrl_i_ctrl_i[cpu_sync_exc] , \ctrl_i_ctrl_i[cpu_trap] , \ctrl_i_ctrl_i[cpu_priv] , \ctrl_i_ctrl_i[ir_rvc] , \ctrl_i_ctrl_i[ir_opcode] , \ctrl_i_ctrl_i[ir_funct12] , \ctrl_i_ctrl_i[ir_funct3] , \ctrl_i_ctrl_i[cnt_event] , \ctrl_i_ctrl_i[csr_wdata] , \ctrl_i_ctrl_i[csr_addr] , \ctrl_i_ctrl_i[csr_re] , \ctrl_i_ctrl_i[csr_we] , \ctrl_i_ctrl_i[lsu_priv] , \ctrl_i_ctrl_i[lsu_mi_en] , \ctrl_i_ctrl_i[lsu_mo_en] , \ctrl_i_ctrl_i[lsu_wr] , \ctrl_i_ctrl_i[lsu_rd] , \ctrl_i_ctrl_i[lsu_req] , \ctrl_i_ctrl_i[alu_cp_fpu] , \ctrl_i_ctrl_i[alu_cp_cfu] , \ctrl_i_ctrl_i[alu_cp_alu] , \ctrl_i_ctrl_i[alu_imm] , \ctrl_i_ctrl_i[alu_unsigned] , \ctrl_i_ctrl_i[alu_opb_mux] , \ctrl_i_ctrl_i[alu_opa_mux] , \ctrl_i_ctrl_i[alu_sub] , \ctrl_i_ctrl_i[alu_op] , \ctrl_i_ctrl_i[rf_zero] , \ctrl_i_ctrl_i[rf_rd] , \ctrl_i_ctrl_i[rf_rs2] , \ctrl_i_ctrl_i[rf_rs1] , \ctrl_i_ctrl_i[rf_wb_en] , \ctrl_i_ctrl_i[pc_ret] , \ctrl_i_ctrl_i[pc_nxt] , \ctrl_i_ctrl_i[pc_cur] , \ctrl_i_ctrl_i[if_ready] , \ctrl_i_ctrl_i[if_reset] };
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:49:10  */
  assign rf_we = n8296; // (signal)
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:50:10  */
  assign addr = n8299; // (signal)
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:67:22  */
  assign n8274 = n8271[98]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8282 = n8271[113]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8284 = 1'b0 | n8282;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8286 = n8271[112]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8287 = n8284 | n8286;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8288 = n8271[111]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8289 = n8287 | n8288;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8290 = n8271[110]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8291 = n8289 | n8290;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8292 = n8271[109]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8293 = n8291 | n8292;
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:67:31  */
  assign n8294 = n8274 & n8293;
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:67:91  */
  assign n8295 = n8271[114]; // extract
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:67:81  */
  assign n8296 = n8294 | n8295;
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:68:43  */
  assign n8298 = n8271[114]; // extract
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:68:30  */
  assign n8299 = n8298 ? 5'b00000 : n8302;
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:69:21  */
  assign n8300 = n8271[113:109]; // extract
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:69:43  */
  assign n8301 = n8271[98]; // extract
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:68:59  */
  assign n8302 = n8301 ? n8300 : n8303;
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:69:71  */
  assign n8303 = n8271[103:99]; // extract
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:79:59  */
  assign n8313 = n8271[108:104]; // extract
  reg [31:0] regfile[31:0] ; // memory
  always @(posedge clk_i)
    if (1'b1)
      n8328 <= regfile[n8313];
  always @(posedge clk_i)
    if (1'b1)
      n8330 <= regfile[addr];
  always @(posedge clk_i)
    if (rf_we)
      regfile[addr] <= rd_i;
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:79:25  */
  /* ../../rtl/core/neorv32_cpu_regfile.vhd:76:19  */
endmodule

module neorv32_cpu_counters_0_64_3c585604e87f855973731fea83e21fab9392d2fc
  (input  clk_i,
   input  rstn_i,
   input  \ctrl_i_ctrl_i[if_reset] ,
   input  \ctrl_i_ctrl_i[if_ready] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_cur] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_nxt] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_ret] ,
   input  \ctrl_i_ctrl_i[rf_wb_en] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs1] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs2] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rd] ,
   input  \ctrl_i_ctrl_i[rf_zero] ,
   input  [2:0] \ctrl_i_ctrl_i[alu_op] ,
   input  \ctrl_i_ctrl_i[alu_sub] ,
   input  \ctrl_i_ctrl_i[alu_opa_mux] ,
   input  \ctrl_i_ctrl_i[alu_opb_mux] ,
   input  \ctrl_i_ctrl_i[alu_unsigned] ,
   input  [31:0] \ctrl_i_ctrl_i[alu_imm] ,
   input  \ctrl_i_ctrl_i[alu_cp_alu] ,
   input  \ctrl_i_ctrl_i[alu_cp_cfu] ,
   input  \ctrl_i_ctrl_i[alu_cp_fpu] ,
   input  \ctrl_i_ctrl_i[lsu_req] ,
   input  \ctrl_i_ctrl_i[lsu_rd] ,
   input  \ctrl_i_ctrl_i[lsu_wr] ,
   input  \ctrl_i_ctrl_i[lsu_mo_en] ,
   input  \ctrl_i_ctrl_i[lsu_mi_en] ,
   input  \ctrl_i_ctrl_i[lsu_priv] ,
   input  \ctrl_i_ctrl_i[csr_we] ,
   input  \ctrl_i_ctrl_i[csr_re] ,
   input  [11:0] \ctrl_i_ctrl_i[csr_addr] ,
   input  [31:0] \ctrl_i_ctrl_i[csr_wdata] ,
   input  [10:0] \ctrl_i_ctrl_i[cnt_event] ,
   input  [2:0] \ctrl_i_ctrl_i[ir_funct3] ,
   input  [11:0] \ctrl_i_ctrl_i[ir_funct12] ,
   input  [6:0] \ctrl_i_ctrl_i[ir_opcode] ,
   input  [15:0] \ctrl_i_ctrl_i[ir_rvc] ,
   input  \ctrl_i_ctrl_i[cpu_priv] ,
   input  \ctrl_i_ctrl_i[cpu_trap] ,
   input  \ctrl_i_ctrl_i[cpu_sync_exc] ,
   input  \ctrl_i_ctrl_i[cpu_debug] ,
   input  [1:0] \ctrl_i_ctrl_i[cpu_fence] ,
   input  [63:0] mtime_i,
   output [31:0] rdata_o);
  wire [263:0] n6020;
  wire [63:0] cnt_we;
  wire cnt_acc;
  wire inh_acc;
  wire [31:0] sel;
  wire [31:0] cnt_re;
  wire [31:0] inhibit;
  wire [31:0] cnt_inc;
  wire [1:0] pmf_inh;
  wire [318:0] hpmevent;
  wire [63:0] rdata64;
  wire [63:0] cycle_rd;
  wire [63:0] time_q;
  wire [63:0] time_rd;
  wire [63:0] instret_rd;
  wire [63:0] hpm_rd;
  wire [63:0] inhibit_rd;
  wire [63:0] pmf_rd;
  wire [4:0] n6023;
  wire n6025;
  wire n6026;
  wire n6028;
  wire n6029;
  wire n6030;
  wire n6031;
  wire n6032;
  wire n6033;
  wire n6034;
  wire n6035;
  wire n6036;
  wire n6037;
  wire n6038;
  wire n6039;
  wire n6040;
  wire n6041;
  wire n6042;
  wire n6043;
  wire n6044;
  wire [4:0] n6054;
  wire n6056;
  wire n6057;
  wire n6059;
  wire n6060;
  wire n6061;
  wire n6062;
  wire n6063;
  wire n6064;
  wire n6065;
  wire n6066;
  wire n6067;
  wire n6068;
  wire n6069;
  wire n6070;
  wire n6071;
  wire n6072;
  wire n6073;
  wire n6074;
  wire n6075;
  wire [4:0] n6085;
  wire n6087;
  wire n6088;
  wire n6090;
  wire n6091;
  wire n6092;
  wire n6093;
  wire n6094;
  wire n6095;
  wire n6096;
  wire n6097;
  wire n6098;
  wire n6099;
  wire n6100;
  wire n6101;
  wire n6102;
  wire n6103;
  wire n6104;
  wire n6105;
  wire n6106;
  wire [4:0] n6116;
  wire n6118;
  wire n6119;
  wire n6121;
  wire n6122;
  wire n6123;
  wire n6124;
  wire n6125;
  wire n6126;
  wire n6127;
  wire n6128;
  wire n6129;
  wire n6130;
  wire n6131;
  wire n6132;
  wire n6133;
  wire n6134;
  wire n6135;
  wire n6136;
  wire n6137;
  wire [4:0] n6147;
  wire n6149;
  wire n6150;
  wire n6152;
  wire n6153;
  wire n6154;
  wire n6155;
  wire n6156;
  wire n6157;
  wire n6158;
  wire n6159;
  wire n6160;
  wire n6161;
  wire n6162;
  wire n6163;
  wire n6164;
  wire n6165;
  wire n6166;
  wire n6167;
  wire n6168;
  wire [4:0] n6178;
  wire n6180;
  wire n6181;
  wire n6183;
  wire n6184;
  wire n6185;
  wire n6186;
  wire n6187;
  wire n6188;
  wire n6189;
  wire n6190;
  wire n6191;
  wire n6192;
  wire n6193;
  wire n6194;
  wire n6195;
  wire n6196;
  wire n6197;
  wire n6198;
  wire n6199;
  wire [4:0] n6209;
  wire n6211;
  wire n6212;
  wire n6214;
  wire n6215;
  wire n6216;
  wire n6217;
  wire n6218;
  wire n6219;
  wire n6220;
  wire n6221;
  wire n6222;
  wire n6223;
  wire n6224;
  wire n6225;
  wire n6226;
  wire n6227;
  wire n6228;
  wire n6229;
  wire n6230;
  wire [4:0] n6240;
  wire n6242;
  wire n6243;
  wire n6245;
  wire n6246;
  wire n6247;
  wire n6248;
  wire n6249;
  wire n6250;
  wire n6251;
  wire n6252;
  wire n6253;
  wire n6254;
  wire n6255;
  wire n6256;
  wire n6257;
  wire n6258;
  wire n6259;
  wire n6260;
  wire n6261;
  wire [4:0] n6271;
  wire n6273;
  wire n6274;
  wire n6276;
  wire n6277;
  wire n6278;
  wire n6279;
  wire n6280;
  wire n6281;
  wire n6282;
  wire n6283;
  wire n6284;
  wire n6285;
  wire n6286;
  wire n6287;
  wire n6288;
  wire n6289;
  wire n6290;
  wire n6291;
  wire n6292;
  wire [4:0] n6302;
  wire n6304;
  wire n6305;
  wire n6307;
  wire n6308;
  wire n6309;
  wire n6310;
  wire n6311;
  wire n6312;
  wire n6313;
  wire n6314;
  wire n6315;
  wire n6316;
  wire n6317;
  wire n6318;
  wire n6319;
  wire n6320;
  wire n6321;
  wire n6322;
  wire n6323;
  wire [4:0] n6333;
  wire n6335;
  wire n6336;
  wire n6338;
  wire n6339;
  wire n6340;
  wire n6341;
  wire n6342;
  wire n6343;
  wire n6344;
  wire n6345;
  wire n6346;
  wire n6347;
  wire n6348;
  wire n6349;
  wire n6350;
  wire n6351;
  wire n6352;
  wire n6353;
  wire n6354;
  wire [4:0] n6364;
  wire n6366;
  wire n6367;
  wire n6369;
  wire n6370;
  wire n6371;
  wire n6372;
  wire n6373;
  wire n6374;
  wire n6375;
  wire n6376;
  wire n6377;
  wire n6378;
  wire n6379;
  wire n6380;
  wire n6381;
  wire n6382;
  wire n6383;
  wire n6384;
  wire n6385;
  wire [4:0] n6395;
  wire n6397;
  wire n6398;
  wire n6400;
  wire n6401;
  wire n6402;
  wire n6403;
  wire n6404;
  wire n6405;
  wire n6406;
  wire n6407;
  wire n6408;
  wire n6409;
  wire n6410;
  wire n6411;
  wire n6412;
  wire n6413;
  wire n6414;
  wire n6415;
  wire n6416;
  wire [4:0] n6426;
  wire n6428;
  wire n6429;
  wire n6431;
  wire n6432;
  wire n6433;
  wire n6434;
  wire n6435;
  wire n6436;
  wire n6437;
  wire n6438;
  wire n6439;
  wire n6440;
  wire n6441;
  wire n6442;
  wire n6443;
  wire n6444;
  wire n6445;
  wire n6446;
  wire n6447;
  wire [4:0] n6457;
  wire n6459;
  wire n6460;
  wire n6462;
  wire n6463;
  wire n6464;
  wire n6465;
  wire n6466;
  wire n6467;
  wire n6468;
  wire n6469;
  wire n6470;
  wire n6471;
  wire n6472;
  wire n6473;
  wire n6474;
  wire n6475;
  wire n6476;
  wire n6477;
  wire n6478;
  wire [4:0] n6488;
  wire n6490;
  wire n6491;
  wire n6493;
  wire n6494;
  wire n6495;
  wire n6496;
  wire n6497;
  wire n6498;
  wire n6499;
  wire n6500;
  wire n6501;
  wire n6502;
  wire n6503;
  wire n6504;
  wire n6505;
  wire n6506;
  wire n6507;
  wire n6508;
  wire n6509;
  wire [4:0] n6519;
  wire n6521;
  wire n6522;
  wire n6524;
  wire n6525;
  wire n6526;
  wire n6527;
  wire n6528;
  wire n6529;
  wire n6530;
  wire n6531;
  wire n6532;
  wire n6533;
  wire n6534;
  wire n6535;
  wire n6536;
  wire n6537;
  wire n6538;
  wire n6539;
  wire n6540;
  wire [4:0] n6550;
  wire n6552;
  wire n6553;
  wire n6555;
  wire n6556;
  wire n6557;
  wire n6558;
  wire n6559;
  wire n6560;
  wire n6561;
  wire n6562;
  wire n6563;
  wire n6564;
  wire n6565;
  wire n6566;
  wire n6567;
  wire n6568;
  wire n6569;
  wire n6570;
  wire n6571;
  wire [4:0] n6581;
  wire n6583;
  wire n6584;
  wire n6586;
  wire n6587;
  wire n6588;
  wire n6589;
  wire n6590;
  wire n6591;
  wire n6592;
  wire n6593;
  wire n6594;
  wire n6595;
  wire n6596;
  wire n6597;
  wire n6598;
  wire n6599;
  wire n6600;
  wire n6601;
  wire n6602;
  wire [4:0] n6612;
  wire n6614;
  wire n6615;
  wire n6617;
  wire n6618;
  wire n6619;
  wire n6620;
  wire n6621;
  wire n6622;
  wire n6623;
  wire n6624;
  wire n6625;
  wire n6626;
  wire n6627;
  wire n6628;
  wire n6629;
  wire n6630;
  wire n6631;
  wire n6632;
  wire n6633;
  wire [4:0] n6643;
  wire n6645;
  wire n6646;
  wire n6648;
  wire n6649;
  wire n6650;
  wire n6651;
  wire n6652;
  wire n6653;
  wire n6654;
  wire n6655;
  wire n6656;
  wire n6657;
  wire n6658;
  wire n6659;
  wire n6660;
  wire n6661;
  wire n6662;
  wire n6663;
  wire n6664;
  wire [4:0] n6674;
  wire n6676;
  wire n6677;
  wire n6679;
  wire n6680;
  wire n6681;
  wire n6682;
  wire n6683;
  wire n6684;
  wire n6685;
  wire n6686;
  wire n6687;
  wire n6688;
  wire n6689;
  wire n6690;
  wire n6691;
  wire n6692;
  wire n6693;
  wire n6694;
  wire n6695;
  wire [4:0] n6705;
  wire n6707;
  wire n6708;
  wire n6710;
  wire n6711;
  wire n6712;
  wire n6713;
  wire n6714;
  wire n6715;
  wire n6716;
  wire n6717;
  wire n6718;
  wire n6719;
  wire n6720;
  wire n6721;
  wire n6722;
  wire n6723;
  wire n6724;
  wire n6725;
  wire n6726;
  wire [4:0] n6736;
  wire n6738;
  wire n6739;
  wire n6741;
  wire n6742;
  wire n6743;
  wire n6744;
  wire n6745;
  wire n6746;
  wire n6747;
  wire n6748;
  wire n6749;
  wire n6750;
  wire n6751;
  wire n6752;
  wire n6753;
  wire n6754;
  wire n6755;
  wire n6756;
  wire n6757;
  wire [4:0] n6767;
  wire n6769;
  wire n6770;
  wire n6772;
  wire n6773;
  wire n6774;
  wire n6775;
  wire n6776;
  wire n6777;
  wire n6778;
  wire n6779;
  wire n6780;
  wire n6781;
  wire n6782;
  wire n6783;
  wire n6784;
  wire n6785;
  wire n6786;
  wire n6787;
  wire n6788;
  wire [4:0] n6798;
  wire n6800;
  wire n6801;
  wire n6803;
  wire n6804;
  wire n6805;
  wire n6806;
  wire n6807;
  wire n6808;
  wire n6809;
  wire n6810;
  wire n6811;
  wire n6812;
  wire n6813;
  wire n6814;
  wire n6815;
  wire n6816;
  wire n6817;
  wire n6818;
  wire n6819;
  wire [4:0] n6829;
  wire n6831;
  wire n6832;
  wire n6834;
  wire n6835;
  wire n6836;
  wire n6837;
  wire n6838;
  wire n6839;
  wire n6840;
  wire n6841;
  wire n6842;
  wire n6843;
  wire n6844;
  wire n6845;
  wire n6846;
  wire n6847;
  wire n6848;
  wire n6849;
  wire n6850;
  wire [4:0] n6860;
  wire n6862;
  wire n6863;
  wire n6865;
  wire n6866;
  wire n6867;
  wire n6868;
  wire n6869;
  wire n6870;
  wire n6871;
  wire n6872;
  wire n6873;
  wire n6874;
  wire n6875;
  wire n6876;
  wire n6877;
  wire n6878;
  wire n6879;
  wire n6880;
  wire n6881;
  wire [4:0] n6891;
  wire n6893;
  wire n6894;
  wire n6896;
  wire n6897;
  wire n6898;
  wire n6899;
  wire n6900;
  wire n6901;
  wire n6902;
  wire n6903;
  wire n6904;
  wire n6905;
  wire n6906;
  wire n6907;
  wire n6908;
  wire n6909;
  wire n6910;
  wire n6911;
  wire n6912;
  wire [4:0] n6922;
  wire n6924;
  wire n6925;
  wire n6927;
  wire n6928;
  wire n6929;
  wire n6930;
  wire n6931;
  wire n6932;
  wire n6933;
  wire n6934;
  wire n6935;
  wire n6936;
  wire n6937;
  wire n6938;
  wire n6939;
  wire n6940;
  wire n6941;
  wire n6942;
  wire n6943;
  wire [4:0] n6953;
  wire n6955;
  wire n6956;
  wire n6958;
  wire n6959;
  wire n6960;
  wire n6961;
  wire n6962;
  wire n6963;
  wire n6964;
  wire n6965;
  wire n6966;
  wire n6967;
  wire n6968;
  wire n6969;
  wire n6970;
  wire n6971;
  wire n6972;
  wire n6973;
  wire n6974;
  wire [4:0] n6984;
  wire n6986;
  wire n6987;
  wire n6989;
  wire n6990;
  wire n6991;
  wire n6992;
  wire n6993;
  wire n6994;
  wire n6995;
  wire n6996;
  wire n6997;
  wire n6998;
  wire n6999;
  wire n7000;
  wire n7001;
  wire n7002;
  wire n7003;
  wire n7004;
  wire n7005;
  wire [6:0] n7015;
  wire n7017;
  wire [6:0] n7018;
  wire n7020;
  wire n7021;
  wire [6:0] n7022;
  wire n7024;
  wire n7025;
  wire [6:0] n7026;
  wire n7028;
  wire n7029;
  wire n7030;
  wire [11:0] n7039;
  wire n7041;
  wire n7042;
  wire [63:0] n7050;
  wire [63:0] n7051;
  wire [63:0] n7052;
  wire [63:0] n7053;
  wire [63:0] n7054;
  wire [31:0] n7055;
  wire n7056;
  wire [31:0] n7057;
  wire [31:0] n7058;
  wire n7060;
  wire n7062;
  wire n7063;
  wire n7064;
  wire n7065;
  wire n7066;
  wire n7067;
  wire n7068;
  wire n7069;
  wire [31:0] n7072;
  wire n7077;
  wire n7078;
  wire [31:0] n7079;
  wire n7086;
  wire n7087;
  wire n7088;
  wire n7089;
  wire n7090;
  wire n7091;
  wire n7092;
  wire n7093;
  wire n7094;
  wire n7095;
  wire n7097;
  wire n7098;
  wire n7099;
  wire n7100;
  wire n7101;
  wire n7102;
  wire n7103;
  wire n7104;
  wire n7105;
  wire n7106;
  wire [10:0] n7108;
  wire [10:0] n7109;
  wire [10:0] n7110;
  wire n7116;
  wire n7118;
  wire n7120;
  wire n7121;
  wire n7122;
  wire n7123;
  wire n7124;
  wire n7125;
  wire n7126;
  wire n7127;
  wire n7128;
  wire n7129;
  wire n7130;
  wire n7131;
  wire n7132;
  wire n7133;
  wire n7134;
  wire n7135;
  wire n7136;
  wire n7137;
  wire n7138;
  wire n7139;
  wire n7140;
  wire n7141;
  wire n7142;
  wire n7143;
  wire n7144;
  wire n7145;
  wire [10:0] n7147;
  wire [10:0] n7148;
  wire [10:0] n7149;
  wire n7155;
  wire n7157;
  wire n7159;
  wire n7160;
  wire n7161;
  wire n7162;
  wire n7163;
  wire n7164;
  wire n7165;
  wire n7166;
  wire n7167;
  wire n7168;
  wire n7169;
  wire n7170;
  wire n7171;
  wire n7172;
  wire n7173;
  wire n7174;
  wire n7175;
  wire n7176;
  wire n7177;
  wire n7178;
  wire n7179;
  wire n7180;
  wire n7181;
  wire n7182;
  wire n7183;
  wire n7184;
  wire [10:0] n7186;
  wire [10:0] n7187;
  wire [10:0] n7188;
  wire n7194;
  wire n7196;
  wire n7198;
  wire n7199;
  wire n7200;
  wire n7201;
  wire n7202;
  wire n7203;
  wire n7204;
  wire n7205;
  wire n7206;
  wire n7207;
  wire n7208;
  wire n7209;
  wire n7210;
  wire n7211;
  wire n7212;
  wire n7213;
  wire n7214;
  wire n7215;
  wire n7216;
  wire n7217;
  wire n7218;
  wire n7219;
  wire n7220;
  wire n7221;
  wire n7222;
  wire n7223;
  wire [10:0] n7225;
  wire [10:0] n7226;
  wire [10:0] n7227;
  wire n7233;
  wire n7235;
  wire n7237;
  wire n7238;
  wire n7239;
  wire n7240;
  wire n7241;
  wire n7242;
  wire n7243;
  wire n7244;
  wire n7245;
  wire n7246;
  wire n7247;
  wire n7248;
  wire n7249;
  wire n7250;
  wire n7251;
  wire n7252;
  wire n7253;
  wire n7254;
  wire n7255;
  wire n7256;
  wire n7257;
  wire n7258;
  wire n7259;
  wire n7260;
  wire n7261;
  wire n7262;
  wire [10:0] n7264;
  wire [10:0] n7265;
  wire [10:0] n7266;
  wire n7272;
  wire n7274;
  wire n7276;
  wire n7277;
  wire n7278;
  wire n7279;
  wire n7280;
  wire n7281;
  wire n7282;
  wire n7283;
  wire n7284;
  wire n7285;
  wire n7286;
  wire n7287;
  wire n7288;
  wire n7289;
  wire n7290;
  wire n7291;
  wire n7292;
  wire n7293;
  wire n7294;
  wire n7295;
  wire n7296;
  wire n7297;
  wire n7298;
  wire n7299;
  wire n7300;
  wire n7301;
  wire [10:0] n7303;
  wire [10:0] n7304;
  wire [10:0] n7305;
  wire n7311;
  wire n7313;
  wire n7315;
  wire n7316;
  wire n7317;
  wire n7318;
  wire n7319;
  wire n7320;
  wire n7321;
  wire n7322;
  wire n7323;
  wire n7324;
  wire n7325;
  wire n7326;
  wire n7327;
  wire n7328;
  wire n7329;
  wire n7330;
  wire n7331;
  wire n7332;
  wire n7333;
  wire n7334;
  wire n7335;
  wire n7336;
  wire n7337;
  wire n7338;
  wire n7339;
  wire n7340;
  wire [10:0] n7342;
  wire [10:0] n7343;
  wire [10:0] n7344;
  wire n7350;
  wire n7352;
  wire n7354;
  wire n7355;
  wire n7356;
  wire n7357;
  wire n7358;
  wire n7359;
  wire n7360;
  wire n7361;
  wire n7362;
  wire n7363;
  wire n7364;
  wire n7365;
  wire n7366;
  wire n7367;
  wire n7368;
  wire n7369;
  wire n7370;
  wire n7371;
  wire n7372;
  wire n7373;
  wire n7374;
  wire n7375;
  wire n7376;
  wire n7377;
  wire n7378;
  wire n7379;
  wire [10:0] n7381;
  wire [10:0] n7382;
  wire [10:0] n7383;
  wire n7389;
  wire n7391;
  wire n7393;
  wire n7394;
  wire n7395;
  wire n7396;
  wire n7397;
  wire n7398;
  wire n7399;
  wire n7400;
  wire n7401;
  wire n7402;
  wire n7403;
  wire n7404;
  wire n7405;
  wire n7406;
  wire n7407;
  wire n7408;
  wire n7409;
  wire n7410;
  wire n7411;
  wire n7412;
  wire n7413;
  wire n7414;
  wire n7415;
  wire n7416;
  wire n7417;
  wire n7418;
  wire [10:0] n7420;
  wire [10:0] n7421;
  wire [10:0] n7422;
  wire n7428;
  wire n7430;
  wire n7432;
  wire n7433;
  wire n7434;
  wire n7435;
  wire n7436;
  wire n7437;
  wire n7438;
  wire n7439;
  wire n7440;
  wire n7441;
  wire n7442;
  wire n7443;
  wire n7444;
  wire n7445;
  wire n7446;
  wire n7447;
  wire n7448;
  wire n7449;
  wire n7450;
  wire n7451;
  wire n7452;
  wire n7453;
  wire n7454;
  wire n7455;
  wire n7456;
  wire n7457;
  wire [10:0] n7459;
  wire [10:0] n7460;
  wire [10:0] n7461;
  wire n7467;
  wire n7469;
  wire n7471;
  wire n7472;
  wire n7473;
  wire n7474;
  wire n7475;
  wire n7476;
  wire n7477;
  wire n7478;
  wire n7479;
  wire n7480;
  wire n7481;
  wire n7482;
  wire n7483;
  wire n7484;
  wire n7485;
  wire n7486;
  wire n7487;
  wire n7488;
  wire n7489;
  wire n7490;
  wire n7491;
  wire n7492;
  wire n7493;
  wire n7494;
  wire n7495;
  wire n7496;
  wire [10:0] n7498;
  wire [10:0] n7499;
  wire [10:0] n7500;
  wire n7506;
  wire n7508;
  wire n7510;
  wire n7511;
  wire n7512;
  wire n7513;
  wire n7514;
  wire n7515;
  wire n7516;
  wire n7517;
  wire n7518;
  wire n7519;
  wire n7520;
  wire n7521;
  wire n7522;
  wire n7523;
  wire n7524;
  wire n7525;
  wire n7526;
  wire n7527;
  wire n7528;
  wire n7529;
  wire n7530;
  wire n7531;
  wire n7532;
  wire n7533;
  wire n7534;
  wire n7535;
  wire [10:0] n7537;
  wire [10:0] n7538;
  wire [10:0] n7539;
  wire n7545;
  wire n7547;
  wire n7549;
  wire n7550;
  wire n7551;
  wire n7552;
  wire n7553;
  wire n7554;
  wire n7555;
  wire n7556;
  wire n7557;
  wire n7558;
  wire n7559;
  wire n7560;
  wire n7561;
  wire n7562;
  wire n7563;
  wire n7564;
  wire n7565;
  wire n7566;
  wire n7567;
  wire n7568;
  wire n7569;
  wire n7570;
  wire n7571;
  wire n7572;
  wire n7573;
  wire n7574;
  wire [10:0] n7576;
  wire [10:0] n7577;
  wire [10:0] n7578;
  wire n7584;
  wire n7586;
  wire n7588;
  wire n7589;
  wire n7590;
  wire n7591;
  wire n7592;
  wire n7593;
  wire n7594;
  wire n7595;
  wire n7596;
  wire n7597;
  wire n7598;
  wire n7599;
  wire n7600;
  wire n7601;
  wire n7602;
  wire n7603;
  wire n7604;
  wire n7605;
  wire n7606;
  wire n7607;
  wire n7608;
  wire n7609;
  wire n7610;
  wire n7611;
  wire n7612;
  wire n7613;
  wire [10:0] n7615;
  wire [10:0] n7616;
  wire [10:0] n7617;
  wire n7623;
  wire n7625;
  wire n7627;
  wire n7628;
  wire n7629;
  wire n7630;
  wire n7631;
  wire n7632;
  wire n7633;
  wire n7634;
  wire n7635;
  wire n7636;
  wire n7637;
  wire n7638;
  wire n7639;
  wire n7640;
  wire n7641;
  wire n7642;
  wire n7643;
  wire n7644;
  wire n7645;
  wire n7646;
  wire n7647;
  wire n7648;
  wire n7649;
  wire n7650;
  wire n7651;
  wire n7652;
  wire [10:0] n7654;
  wire [10:0] n7655;
  wire [10:0] n7656;
  wire n7662;
  wire n7664;
  wire n7666;
  wire n7667;
  wire n7668;
  wire n7669;
  wire n7670;
  wire n7671;
  wire n7672;
  wire n7673;
  wire n7674;
  wire n7675;
  wire n7676;
  wire n7677;
  wire n7678;
  wire n7679;
  wire n7680;
  wire n7681;
  wire n7682;
  wire n7683;
  wire n7684;
  wire n7685;
  wire n7686;
  wire n7687;
  wire n7688;
  wire n7689;
  wire n7690;
  wire n7691;
  wire [10:0] n7693;
  wire [10:0] n7694;
  wire [10:0] n7695;
  wire n7701;
  wire n7703;
  wire n7705;
  wire n7706;
  wire n7707;
  wire n7708;
  wire n7709;
  wire n7710;
  wire n7711;
  wire n7712;
  wire n7713;
  wire n7714;
  wire n7715;
  wire n7716;
  wire n7717;
  wire n7718;
  wire n7719;
  wire n7720;
  wire n7721;
  wire n7722;
  wire n7723;
  wire n7724;
  wire n7725;
  wire n7726;
  wire n7727;
  wire n7728;
  wire n7729;
  wire n7730;
  wire [10:0] n7732;
  wire [10:0] n7733;
  wire [10:0] n7734;
  wire n7740;
  wire n7742;
  wire n7744;
  wire n7745;
  wire n7746;
  wire n7747;
  wire n7748;
  wire n7749;
  wire n7750;
  wire n7751;
  wire n7752;
  wire n7753;
  wire n7754;
  wire n7755;
  wire n7756;
  wire n7757;
  wire n7758;
  wire n7759;
  wire n7760;
  wire n7761;
  wire n7762;
  wire n7763;
  wire n7764;
  wire n7765;
  wire n7766;
  wire n7767;
  wire n7768;
  wire n7769;
  wire [10:0] n7771;
  wire [10:0] n7772;
  wire [10:0] n7773;
  wire n7779;
  wire n7781;
  wire n7783;
  wire n7784;
  wire n7785;
  wire n7786;
  wire n7787;
  wire n7788;
  wire n7789;
  wire n7790;
  wire n7791;
  wire n7792;
  wire n7793;
  wire n7794;
  wire n7795;
  wire n7796;
  wire n7797;
  wire n7798;
  wire n7799;
  wire n7800;
  wire n7801;
  wire n7802;
  wire n7803;
  wire n7804;
  wire n7805;
  wire n7806;
  wire n7807;
  wire n7808;
  wire [10:0] n7810;
  wire [10:0] n7811;
  wire [10:0] n7812;
  wire n7818;
  wire n7820;
  wire n7822;
  wire n7823;
  wire n7824;
  wire n7825;
  wire n7826;
  wire n7827;
  wire n7828;
  wire n7829;
  wire n7830;
  wire n7831;
  wire n7832;
  wire n7833;
  wire n7834;
  wire n7835;
  wire n7836;
  wire n7837;
  wire n7838;
  wire n7839;
  wire n7840;
  wire n7841;
  wire n7842;
  wire n7843;
  wire n7844;
  wire n7845;
  wire n7846;
  wire n7847;
  wire [10:0] n7849;
  wire [10:0] n7850;
  wire [10:0] n7851;
  wire n7857;
  wire n7859;
  wire n7861;
  wire n7862;
  wire n7863;
  wire n7864;
  wire n7865;
  wire n7866;
  wire n7867;
  wire n7868;
  wire n7869;
  wire n7870;
  wire n7871;
  wire n7872;
  wire n7873;
  wire n7874;
  wire n7875;
  wire n7876;
  wire n7877;
  wire n7878;
  wire n7879;
  wire n7880;
  wire n7881;
  wire n7882;
  wire n7883;
  wire n7884;
  wire n7885;
  wire n7886;
  wire [10:0] n7888;
  wire [10:0] n7889;
  wire [10:0] n7890;
  wire n7896;
  wire n7898;
  wire n7900;
  wire n7901;
  wire n7902;
  wire n7903;
  wire n7904;
  wire n7905;
  wire n7906;
  wire n7907;
  wire n7908;
  wire n7909;
  wire n7910;
  wire n7911;
  wire n7912;
  wire n7913;
  wire n7914;
  wire n7915;
  wire n7916;
  wire n7917;
  wire n7918;
  wire n7919;
  wire n7920;
  wire n7921;
  wire n7922;
  wire n7923;
  wire n7924;
  wire n7925;
  wire [10:0] n7927;
  wire [10:0] n7928;
  wire [10:0] n7929;
  wire n7935;
  wire n7937;
  wire n7939;
  wire n7940;
  wire n7941;
  wire n7942;
  wire n7943;
  wire n7944;
  wire n7945;
  wire n7946;
  wire n7947;
  wire n7948;
  wire n7949;
  wire n7950;
  wire n7951;
  wire n7952;
  wire n7953;
  wire n7954;
  wire n7955;
  wire n7956;
  wire n7957;
  wire n7958;
  wire n7959;
  wire n7960;
  wire n7961;
  wire n7962;
  wire n7963;
  wire n7964;
  wire [10:0] n7966;
  wire [10:0] n7967;
  wire [10:0] n7968;
  wire n7974;
  wire n7976;
  wire n7978;
  wire n7979;
  wire n7980;
  wire n7981;
  wire n7982;
  wire n7983;
  wire n7984;
  wire n7985;
  wire n7986;
  wire n7987;
  wire n7988;
  wire n7989;
  wire n7990;
  wire n7991;
  wire n7992;
  wire n7993;
  wire n7994;
  wire n7995;
  wire n7996;
  wire n7997;
  wire n7998;
  wire n7999;
  wire n8000;
  wire n8001;
  wire n8002;
  wire n8003;
  wire [10:0] n8005;
  wire [10:0] n8006;
  wire [10:0] n8007;
  wire n8013;
  wire n8015;
  wire n8017;
  wire n8018;
  wire n8019;
  wire n8020;
  wire n8021;
  wire n8022;
  wire n8023;
  wire n8024;
  wire n8025;
  wire n8026;
  wire n8027;
  wire n8028;
  wire n8029;
  wire n8030;
  wire n8031;
  wire n8032;
  wire n8033;
  wire n8034;
  wire n8035;
  wire n8036;
  wire n8037;
  wire n8038;
  wire n8039;
  wire n8040;
  wire n8041;
  wire n8042;
  wire [10:0] n8044;
  wire [10:0] n8045;
  wire [10:0] n8046;
  wire n8052;
  wire n8054;
  wire n8056;
  wire n8057;
  wire n8058;
  wire n8059;
  wire n8060;
  wire n8061;
  wire n8062;
  wire n8063;
  wire n8064;
  wire n8065;
  wire n8066;
  wire n8067;
  wire n8068;
  wire n8069;
  wire n8070;
  wire n8071;
  wire n8072;
  wire n8073;
  wire n8074;
  wire n8075;
  wire n8076;
  wire n8077;
  wire n8078;
  wire n8079;
  wire n8080;
  wire n8081;
  wire [10:0] n8083;
  wire [10:0] n8084;
  wire [10:0] n8085;
  wire n8091;
  wire n8093;
  wire n8095;
  wire n8096;
  wire n8097;
  wire n8098;
  wire n8099;
  wire n8100;
  wire n8101;
  wire n8102;
  wire n8103;
  wire n8104;
  wire n8105;
  wire n8106;
  wire n8107;
  wire n8108;
  wire n8109;
  wire n8110;
  wire n8111;
  wire n8112;
  wire n8113;
  wire n8114;
  wire n8115;
  wire n8116;
  wire n8117;
  wire n8118;
  wire n8119;
  wire n8120;
  wire [10:0] n8122;
  wire [10:0] n8123;
  wire [10:0] n8124;
  wire n8130;
  wire n8132;
  wire n8134;
  wire n8135;
  wire n8136;
  wire n8137;
  wire n8138;
  wire n8139;
  wire n8140;
  wire n8141;
  wire n8142;
  wire n8143;
  wire n8144;
  wire n8145;
  wire n8146;
  wire n8147;
  wire n8148;
  wire n8149;
  wire n8150;
  wire n8151;
  wire n8152;
  wire n8153;
  wire n8154;
  wire n8155;
  wire n8156;
  wire n8157;
  wire n8158;
  wire n8159;
  wire [10:0] n8161;
  wire [10:0] n8162;
  wire [10:0] n8163;
  wire n8169;
  wire n8171;
  wire n8173;
  wire n8174;
  wire n8175;
  wire n8176;
  wire n8177;
  wire n8178;
  wire n8179;
  wire n8180;
  wire n8181;
  wire n8182;
  wire n8183;
  wire n8184;
  wire n8185;
  wire n8186;
  wire n8187;
  wire n8188;
  wire n8189;
  wire n8190;
  wire n8191;
  wire n8192;
  wire n8193;
  wire n8194;
  wire n8195;
  wire n8196;
  wire n8197;
  wire n8198;
  wire [10:0] n8200;
  wire [10:0] n8201;
  wire [10:0] n8202;
  wire n8208;
  wire n8210;
  wire n8212;
  wire n8213;
  wire n8214;
  wire n8215;
  wire n8216;
  wire n8217;
  wire n8218;
  wire n8219;
  wire n8220;
  wire n8221;
  wire n8222;
  wire n8223;
  wire n8224;
  wire n8225;
  wire n8226;
  wire n8227;
  wire n8228;
  wire n8229;
  wire n8230;
  wire n8231;
  wire n8232;
  wire n8233;
  wire n8234;
  wire n8235;
  wire n8236;
  wire n8237;
  wire n8238;
  wire [1:0] n8239;
  wire [31:0] n8240;
  wire n8241;
  wire n8244;
  wire n8250;
  wire [63:0] n8251;
  wire n8253;
  wire [1:0] n8254;
  wire [31:0] n8255;
  wire n8256;
  wire [63:0] n8262;
  wire [31:0] n8263;
  wire [31:0] n8264;
  reg [31:0] n8267;
  wire [31:0] n8268;
  reg [63:0] n8269;
  wire [63:0] n8270;
  assign rdata_o = n7057; //(module output)
  assign n6020 = {\ctrl_i_ctrl_i[cpu_fence] , \ctrl_i_ctrl_i[cpu_debug] , \ctrl_i_ctrl_i[cpu_sync_exc] , \ctrl_i_ctrl_i[cpu_trap] , \ctrl_i_ctrl_i[cpu_priv] , \ctrl_i_ctrl_i[ir_rvc] , \ctrl_i_ctrl_i[ir_opcode] , \ctrl_i_ctrl_i[ir_funct12] , \ctrl_i_ctrl_i[ir_funct3] , \ctrl_i_ctrl_i[cnt_event] , \ctrl_i_ctrl_i[csr_wdata] , \ctrl_i_ctrl_i[csr_addr] , \ctrl_i_ctrl_i[csr_re] , \ctrl_i_ctrl_i[csr_we] , \ctrl_i_ctrl_i[lsu_priv] , \ctrl_i_ctrl_i[lsu_mi_en] , \ctrl_i_ctrl_i[lsu_mo_en] , \ctrl_i_ctrl_i[lsu_wr] , \ctrl_i_ctrl_i[lsu_rd] , \ctrl_i_ctrl_i[lsu_req] , \ctrl_i_ctrl_i[alu_cp_fpu] , \ctrl_i_ctrl_i[alu_cp_cfu] , \ctrl_i_ctrl_i[alu_cp_alu] , \ctrl_i_ctrl_i[alu_imm] , \ctrl_i_ctrl_i[alu_unsigned] , \ctrl_i_ctrl_i[alu_opb_mux] , \ctrl_i_ctrl_i[alu_opa_mux] , \ctrl_i_ctrl_i[alu_sub] , \ctrl_i_ctrl_i[alu_op] , \ctrl_i_ctrl_i[rf_zero] , \ctrl_i_ctrl_i[rf_rd] , \ctrl_i_ctrl_i[rf_rs2] , \ctrl_i_ctrl_i[rf_rs1] , \ctrl_i_ctrl_i[rf_wb_en] , \ctrl_i_ctrl_i[pc_ret] , \ctrl_i_ctrl_i[pc_nxt] , \ctrl_i_ctrl_i[pc_cur] , \ctrl_i_ctrl_i[if_ready] , \ctrl_i_ctrl_i[if_reset] };
  /* ../../rtl/core/neorv32_cpu_counters.vhd:48:10  */
  assign cnt_we = n8262; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:49:10  */
  assign cnt_acc = n7030; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:49:28  */
  assign inh_acc = n7042; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:50:10  */
  assign sel = n8263; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:50:15  */
  assign cnt_re = n8264; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:53:10  */
  assign inhibit = n8267; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:53:19  */
  assign cnt_inc = n8268; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:54:26  */
  assign pmf_inh = 2'b00; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:59:10  */
  assign hpmevent = 319'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:63:10  */
  assign rdata64 = n7054; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:63:29  */
  assign time_q = n8269; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:63:37  */
  assign time_rd = n8251; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:63:58  */
  assign hpm_rd = 64'b0000000000000000000000000000000000000000000000000000000000000000; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:63:66  */
  assign inhibit_rd = n8270; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:63:78  */
  assign pmf_rd = 64'b0000000000000000000000000000000000000000000000000000000000000000; // (signal)
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6023 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6025 = n6023 == 5'b00000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6026 = n6025 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6028 = sel[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6029 = cnt_acc & n6028;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6030 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6031 = n6029 & n6030;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6032 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6033 = ~n6032;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6034 = n6031 & n6033;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6035 = sel[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6036 = cnt_acc & n6035;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6037 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6038 = n6036 & n6037;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6039 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6040 = n6038 & n6039;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6041 = sel[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6042 = cnt_acc & n6041;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6043 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6044 = n6042 & n6043;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6054 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6056 = n6054 == 5'b00001;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6057 = n6056 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6059 = sel[1]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6060 = cnt_acc & n6059;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6061 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6062 = n6060 & n6061;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6063 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6064 = ~n6063;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6065 = n6062 & n6064;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6066 = sel[1]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6067 = cnt_acc & n6066;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6068 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6069 = n6067 & n6068;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6070 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6071 = n6069 & n6070;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6072 = sel[1]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6073 = cnt_acc & n6072;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6074 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6075 = n6073 & n6074;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6085 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6087 = n6085 == 5'b00010;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6088 = n6087 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6090 = sel[2]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6091 = cnt_acc & n6090;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6092 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6093 = n6091 & n6092;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6094 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6095 = ~n6094;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6096 = n6093 & n6095;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6097 = sel[2]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6098 = cnt_acc & n6097;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6099 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6100 = n6098 & n6099;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6101 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6102 = n6100 & n6101;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6103 = sel[2]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6104 = cnt_acc & n6103;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6105 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6106 = n6104 & n6105;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6116 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6118 = n6116 == 5'b00011;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6119 = n6118 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6121 = sel[3]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6122 = cnt_acc & n6121;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6123 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6124 = n6122 & n6123;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6125 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6126 = ~n6125;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6127 = n6124 & n6126;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6128 = sel[3]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6129 = cnt_acc & n6128;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6130 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6131 = n6129 & n6130;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6132 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6133 = n6131 & n6132;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6134 = sel[3]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6135 = cnt_acc & n6134;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6136 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6137 = n6135 & n6136;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6147 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6149 = n6147 == 5'b00100;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6150 = n6149 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6152 = sel[4]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6153 = cnt_acc & n6152;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6154 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6155 = n6153 & n6154;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6156 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6157 = ~n6156;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6158 = n6155 & n6157;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6159 = sel[4]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6160 = cnt_acc & n6159;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6161 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6162 = n6160 & n6161;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6163 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6164 = n6162 & n6163;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6165 = sel[4]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6166 = cnt_acc & n6165;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6167 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6168 = n6166 & n6167;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6178 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6180 = n6178 == 5'b00101;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6181 = n6180 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6183 = sel[5]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6184 = cnt_acc & n6183;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6185 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6186 = n6184 & n6185;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6187 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6188 = ~n6187;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6189 = n6186 & n6188;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6190 = sel[5]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6191 = cnt_acc & n6190;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6192 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6193 = n6191 & n6192;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6194 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6195 = n6193 & n6194;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6196 = sel[5]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6197 = cnt_acc & n6196;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6198 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6199 = n6197 & n6198;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6209 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6211 = n6209 == 5'b00110;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6212 = n6211 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6214 = sel[6]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6215 = cnt_acc & n6214;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6216 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6217 = n6215 & n6216;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6218 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6219 = ~n6218;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6220 = n6217 & n6219;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6221 = sel[6]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6222 = cnt_acc & n6221;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6223 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6224 = n6222 & n6223;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6225 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6226 = n6224 & n6225;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6227 = sel[6]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6228 = cnt_acc & n6227;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6229 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6230 = n6228 & n6229;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6240 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6242 = n6240 == 5'b00111;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6243 = n6242 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6245 = sel[7]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6246 = cnt_acc & n6245;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6247 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6248 = n6246 & n6247;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6249 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6250 = ~n6249;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6251 = n6248 & n6250;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6252 = sel[7]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6253 = cnt_acc & n6252;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6254 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6255 = n6253 & n6254;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6256 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6257 = n6255 & n6256;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6258 = sel[7]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6259 = cnt_acc & n6258;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6260 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6261 = n6259 & n6260;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6271 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6273 = n6271 == 5'b01000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6274 = n6273 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6276 = sel[8]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6277 = cnt_acc & n6276;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6278 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6279 = n6277 & n6278;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6280 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6281 = ~n6280;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6282 = n6279 & n6281;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6283 = sel[8]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6284 = cnt_acc & n6283;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6285 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6286 = n6284 & n6285;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6287 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6288 = n6286 & n6287;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6289 = sel[8]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6290 = cnt_acc & n6289;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6291 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6292 = n6290 & n6291;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6302 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6304 = n6302 == 5'b01001;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6305 = n6304 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6307 = sel[9]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6308 = cnt_acc & n6307;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6309 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6310 = n6308 & n6309;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6311 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6312 = ~n6311;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6313 = n6310 & n6312;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6314 = sel[9]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6315 = cnt_acc & n6314;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6316 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6317 = n6315 & n6316;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6318 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6319 = n6317 & n6318;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6320 = sel[9]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6321 = cnt_acc & n6320;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6322 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6323 = n6321 & n6322;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6333 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6335 = n6333 == 5'b01010;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6336 = n6335 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6338 = sel[10]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6339 = cnt_acc & n6338;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6340 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6341 = n6339 & n6340;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6342 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6343 = ~n6342;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6344 = n6341 & n6343;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6345 = sel[10]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6346 = cnt_acc & n6345;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6347 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6348 = n6346 & n6347;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6349 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6350 = n6348 & n6349;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6351 = sel[10]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6352 = cnt_acc & n6351;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6353 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6354 = n6352 & n6353;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6364 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6366 = n6364 == 5'b01011;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6367 = n6366 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6369 = sel[11]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6370 = cnt_acc & n6369;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6371 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6372 = n6370 & n6371;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6373 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6374 = ~n6373;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6375 = n6372 & n6374;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6376 = sel[11]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6377 = cnt_acc & n6376;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6378 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6379 = n6377 & n6378;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6380 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6381 = n6379 & n6380;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6382 = sel[11]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6383 = cnt_acc & n6382;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6384 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6385 = n6383 & n6384;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6395 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6397 = n6395 == 5'b01100;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6398 = n6397 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6400 = sel[12]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6401 = cnt_acc & n6400;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6402 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6403 = n6401 & n6402;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6404 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6405 = ~n6404;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6406 = n6403 & n6405;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6407 = sel[12]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6408 = cnt_acc & n6407;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6409 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6410 = n6408 & n6409;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6411 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6412 = n6410 & n6411;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6413 = sel[12]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6414 = cnt_acc & n6413;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6415 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6416 = n6414 & n6415;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6426 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6428 = n6426 == 5'b01101;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6429 = n6428 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6431 = sel[13]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6432 = cnt_acc & n6431;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6433 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6434 = n6432 & n6433;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6435 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6436 = ~n6435;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6437 = n6434 & n6436;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6438 = sel[13]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6439 = cnt_acc & n6438;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6440 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6441 = n6439 & n6440;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6442 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6443 = n6441 & n6442;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6444 = sel[13]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6445 = cnt_acc & n6444;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6446 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6447 = n6445 & n6446;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6457 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6459 = n6457 == 5'b01110;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6460 = n6459 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6462 = sel[14]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6463 = cnt_acc & n6462;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6464 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6465 = n6463 & n6464;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6466 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6467 = ~n6466;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6468 = n6465 & n6467;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6469 = sel[14]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6470 = cnt_acc & n6469;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6471 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6472 = n6470 & n6471;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6473 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6474 = n6472 & n6473;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6475 = sel[14]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6476 = cnt_acc & n6475;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6477 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6478 = n6476 & n6477;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6488 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6490 = n6488 == 5'b01111;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6491 = n6490 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6493 = sel[15]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6494 = cnt_acc & n6493;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6495 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6496 = n6494 & n6495;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6497 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6498 = ~n6497;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6499 = n6496 & n6498;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6500 = sel[15]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6501 = cnt_acc & n6500;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6502 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6503 = n6501 & n6502;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6504 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6505 = n6503 & n6504;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6506 = sel[15]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6507 = cnt_acc & n6506;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6508 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6509 = n6507 & n6508;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6519 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6521 = n6519 == 5'b10000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6522 = n6521 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6524 = sel[16]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6525 = cnt_acc & n6524;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6526 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6527 = n6525 & n6526;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6528 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6529 = ~n6528;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6530 = n6527 & n6529;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6531 = sel[16]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6532 = cnt_acc & n6531;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6533 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6534 = n6532 & n6533;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6535 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6536 = n6534 & n6535;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6537 = sel[16]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6538 = cnt_acc & n6537;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6539 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6540 = n6538 & n6539;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6550 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6552 = n6550 == 5'b10001;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6553 = n6552 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6555 = sel[17]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6556 = cnt_acc & n6555;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6557 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6558 = n6556 & n6557;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6559 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6560 = ~n6559;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6561 = n6558 & n6560;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6562 = sel[17]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6563 = cnt_acc & n6562;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6564 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6565 = n6563 & n6564;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6566 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6567 = n6565 & n6566;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6568 = sel[17]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6569 = cnt_acc & n6568;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6570 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6571 = n6569 & n6570;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6581 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6583 = n6581 == 5'b10010;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6584 = n6583 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6586 = sel[18]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6587 = cnt_acc & n6586;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6588 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6589 = n6587 & n6588;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6590 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6591 = ~n6590;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6592 = n6589 & n6591;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6593 = sel[18]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6594 = cnt_acc & n6593;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6595 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6596 = n6594 & n6595;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6597 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6598 = n6596 & n6597;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6599 = sel[18]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6600 = cnt_acc & n6599;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6601 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6602 = n6600 & n6601;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6612 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6614 = n6612 == 5'b10011;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6615 = n6614 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6617 = sel[19]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6618 = cnt_acc & n6617;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6619 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6620 = n6618 & n6619;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6621 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6622 = ~n6621;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6623 = n6620 & n6622;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6624 = sel[19]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6625 = cnt_acc & n6624;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6626 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6627 = n6625 & n6626;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6628 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6629 = n6627 & n6628;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6630 = sel[19]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6631 = cnt_acc & n6630;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6632 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6633 = n6631 & n6632;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6643 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6645 = n6643 == 5'b10100;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6646 = n6645 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6648 = sel[20]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6649 = cnt_acc & n6648;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6650 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6651 = n6649 & n6650;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6652 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6653 = ~n6652;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6654 = n6651 & n6653;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6655 = sel[20]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6656 = cnt_acc & n6655;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6657 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6658 = n6656 & n6657;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6659 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6660 = n6658 & n6659;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6661 = sel[20]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6662 = cnt_acc & n6661;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6663 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6664 = n6662 & n6663;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6674 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6676 = n6674 == 5'b10101;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6677 = n6676 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6679 = sel[21]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6680 = cnt_acc & n6679;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6681 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6682 = n6680 & n6681;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6683 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6684 = ~n6683;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6685 = n6682 & n6684;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6686 = sel[21]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6687 = cnt_acc & n6686;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6688 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6689 = n6687 & n6688;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6690 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6691 = n6689 & n6690;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6692 = sel[21]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6693 = cnt_acc & n6692;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6694 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6695 = n6693 & n6694;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6705 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6707 = n6705 == 5'b10110;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6708 = n6707 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6710 = sel[22]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6711 = cnt_acc & n6710;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6712 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6713 = n6711 & n6712;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6714 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6715 = ~n6714;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6716 = n6713 & n6715;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6717 = sel[22]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6718 = cnt_acc & n6717;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6719 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6720 = n6718 & n6719;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6721 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6722 = n6720 & n6721;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6723 = sel[22]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6724 = cnt_acc & n6723;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6725 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6726 = n6724 & n6725;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6736 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6738 = n6736 == 5'b10111;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6739 = n6738 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6741 = sel[23]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6742 = cnt_acc & n6741;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6743 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6744 = n6742 & n6743;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6745 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6746 = ~n6745;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6747 = n6744 & n6746;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6748 = sel[23]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6749 = cnt_acc & n6748;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6750 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6751 = n6749 & n6750;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6752 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6753 = n6751 & n6752;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6754 = sel[23]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6755 = cnt_acc & n6754;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6756 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6757 = n6755 & n6756;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6767 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6769 = n6767 == 5'b11000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6770 = n6769 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6772 = sel[24]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6773 = cnt_acc & n6772;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6774 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6775 = n6773 & n6774;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6776 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6777 = ~n6776;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6778 = n6775 & n6777;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6779 = sel[24]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6780 = cnt_acc & n6779;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6781 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6782 = n6780 & n6781;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6783 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6784 = n6782 & n6783;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6785 = sel[24]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6786 = cnt_acc & n6785;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6787 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6788 = n6786 & n6787;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6798 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6800 = n6798 == 5'b11001;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6801 = n6800 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6803 = sel[25]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6804 = cnt_acc & n6803;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6805 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6806 = n6804 & n6805;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6807 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6808 = ~n6807;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6809 = n6806 & n6808;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6810 = sel[25]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6811 = cnt_acc & n6810;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6812 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6813 = n6811 & n6812;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6814 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6815 = n6813 & n6814;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6816 = sel[25]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6817 = cnt_acc & n6816;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6818 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6819 = n6817 & n6818;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6829 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6831 = n6829 == 5'b11010;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6832 = n6831 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6834 = sel[26]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6835 = cnt_acc & n6834;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6836 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6837 = n6835 & n6836;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6838 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6839 = ~n6838;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6840 = n6837 & n6839;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6841 = sel[26]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6842 = cnt_acc & n6841;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6843 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6844 = n6842 & n6843;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6845 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6846 = n6844 & n6845;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6847 = sel[26]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6848 = cnt_acc & n6847;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6849 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6850 = n6848 & n6849;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6860 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6862 = n6860 == 5'b11011;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6863 = n6862 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6865 = sel[27]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6866 = cnt_acc & n6865;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6867 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6868 = n6866 & n6867;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6869 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6870 = ~n6869;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6871 = n6868 & n6870;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6872 = sel[27]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6873 = cnt_acc & n6872;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6874 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6875 = n6873 & n6874;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6876 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6877 = n6875 & n6876;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6878 = sel[27]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6879 = cnt_acc & n6878;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6880 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6881 = n6879 & n6880;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6891 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6893 = n6891 == 5'b11100;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6894 = n6893 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6896 = sel[28]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6897 = cnt_acc & n6896;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6898 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6899 = n6897 & n6898;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6900 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6901 = ~n6900;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6902 = n6899 & n6901;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6903 = sel[28]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6904 = cnt_acc & n6903;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6905 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6906 = n6904 & n6905;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6907 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6908 = n6906 & n6907;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6909 = sel[28]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6910 = cnt_acc & n6909;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6911 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6912 = n6910 & n6911;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6922 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6924 = n6922 == 5'b11101;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6925 = n6924 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6927 = sel[29]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6928 = cnt_acc & n6927;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6929 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6930 = n6928 & n6929;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6931 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6932 = ~n6931;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6933 = n6930 & n6932;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6934 = sel[29]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6935 = cnt_acc & n6934;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6936 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6937 = n6935 & n6936;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6938 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6939 = n6937 & n6938;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6940 = sel[29]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6941 = cnt_acc & n6940;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6942 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6943 = n6941 & n6942;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6953 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6955 = n6953 == 5'b11110;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6956 = n6955 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6958 = sel[30]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6959 = cnt_acc & n6958;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6960 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6961 = n6959 & n6960;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6962 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6963 = ~n6962;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6964 = n6961 & n6963;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6965 = sel[30]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6966 = cnt_acc & n6965;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6967 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6968 = n6966 & n6967;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n6969 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n6970 = n6968 & n6969;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n6971 = sel[30]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n6972 = cnt_acc & n6971;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n6973 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n6974 = n6972 & n6973;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:46  */
  assign n6984 = n6020[169:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:59  */
  assign n6986 = n6984 == 5'b11111;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:71:25  */
  assign n6987 = n6986 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:36  */
  assign n6989 = sel[31]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:29  */
  assign n6990 = cnt_acc & n6989;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:51  */
  assign n6991 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:40  */
  assign n6992 = n6990 & n6991;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:82  */
  assign n6993 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:63  */
  assign n6994 = ~n6993;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:72:58  */
  assign n6995 = n6992 & n6994;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:36  */
  assign n6996 = sel[31]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:29  */
  assign n6997 = cnt_acc & n6996;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:51  */
  assign n6998 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:40  */
  assign n6999 = n6997 & n6998;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:82  */
  assign n7000 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:73:58  */
  assign n7001 = n6999 & n7000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:36  */
  assign n7002 = sel[31]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:29  */
  assign n7003 = cnt_acc & n7002;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:51  */
  assign n7004 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:74:40  */
  assign n7005 = n7003 & n7004;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:80:40  */
  assign n7015 = n6020[176:170]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:80:54  */
  assign n7017 = n7015 == 7'b1100000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:81:40  */
  assign n7018 = n6020[176:170]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:81:54  */
  assign n7020 = n7018 == 7'b1011000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:80:82  */
  assign n7021 = n7017 | n7020;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:82:40  */
  assign n7022 = n6020[176:170]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:82:54  */
  assign n7024 = n7022 == 7'b1100100;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:81:83  */
  assign n7025 = n7021 | n7024;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:83:40  */
  assign n7026 = n6020[176:170]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:83:54  */
  assign n7028 = n7026 == 7'b1011100;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:82:83  */
  assign n7029 = n7025 | n7028;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:80:18  */
  assign n7030 = n7029 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:85:31  */
  assign n7039 = n6020[176:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:85:40  */
  assign n7041 = n7039 == 12'b001100100000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:85:18  */
  assign n7042 = n7041 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:89:23  */
  assign n7050 = cycle_rd | time_rd;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:89:34  */
  assign n7051 = n7050 | instret_rd;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:89:48  */
  assign n7052 = n7051 | hpm_rd;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:89:58  */
  assign n7053 = n7052 | inhibit_rd;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:89:72  */
  assign n7054 = n7053 | pmf_rd;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:90:21  */
  assign n7055 = rdata64[63:32]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:90:57  */
  assign n7056 = n6020[172]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:90:36  */
  assign n7057 = n7056 ? n7055 : n7058;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:90:80  */
  assign n7058 = rdata64[31:0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:97:16  */
  assign n7060 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:104:41  */
  assign n7062 = n6020[163]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:104:29  */
  assign n7063 = n7062 & inh_acc;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:105:39  */
  assign n7064 = n6020[177]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:106:39  */
  assign n7065 = n6020[179]; // extract
  assign n7066 = inhibit[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:104:7  */
  assign n7067 = n7063 ? n7064 : n7066;
  assign n7068 = inhibit[2]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:104:7  */
  assign n7069 = n7063 ? n7065 : n7068;
  assign n7072 = {29'b00000000000000000000000000000, n7069, 1'b0, n7067};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:120:72  */
  assign n7077 = n6020[164]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:120:60  */
  assign n7078 = n7077 & inh_acc;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:120:39  */
  assign n7079 = n7078 ? inhibit : 32'b00000000000000000000000000000000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:33  */
  assign n7086 = n6020[209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:66  */
  assign n7087 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:55  */
  assign n7088 = ~n7087;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:50  */
  assign n7089 = n7086 & n7088;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:93  */
  assign n7090 = inhibit[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:82  */
  assign n7091 = ~n7090;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:77  */
  assign n7092 = n7089 & n7091;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:114  */
  assign n7093 = pmf_inh[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:103  */
  assign n7094 = ~n7093;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:183:98  */
  assign n7095 = n7092 & n7094;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:33  */
  assign n7097 = n6020[211]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:66  */
  assign n7098 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:55  */
  assign n7099 = ~n7098;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:50  */
  assign n7100 = n7097 & n7099;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:93  */
  assign n7101 = inhibit[2]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:82  */
  assign n7102 = ~n7101;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:77  */
  assign n7103 = n7100 & n7102;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:114  */
  assign n7104 = pmf_inh[1]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:103  */
  assign n7105 = ~n7104;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:185:98  */
  assign n7106 = n7103 & n7105;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7108 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7109 = hpmevent[318:308]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7110 = n7108 & n7109;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7116 = n7110[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7118 = 1'b0 | n7116;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7120 = n7110[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7121 = n7118 | n7120;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7122 = n7110[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7123 = n7121 | n7122;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7124 = n7110[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7125 = n7123 | n7124;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7126 = n7110[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7127 = n7125 | n7126;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7128 = n7110[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7129 = n7127 | n7128;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7130 = n7110[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7131 = n7129 | n7130;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7132 = n7110[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7133 = n7131 | n7132;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7134 = n7110[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7135 = n7133 | n7134;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7136 = n7110[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7137 = n7135 | n7136;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7138 = n7110[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7139 = n7137 | n7138;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7140 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7141 = ~n7140;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7142 = n7139 & n7141;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7143 = inhibit[3]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7144 = ~n7143;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7145 = n7142 & n7144;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7147 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7148 = hpmevent[307:297]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7149 = n7147 & n7148;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7155 = n7149[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7157 = 1'b0 | n7155;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7159 = n7149[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7160 = n7157 | n7159;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7161 = n7149[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7162 = n7160 | n7161;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7163 = n7149[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7164 = n7162 | n7163;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7165 = n7149[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7166 = n7164 | n7165;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7167 = n7149[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7168 = n7166 | n7167;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7169 = n7149[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7170 = n7168 | n7169;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7171 = n7149[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7172 = n7170 | n7171;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7173 = n7149[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7174 = n7172 | n7173;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7175 = n7149[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7176 = n7174 | n7175;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7177 = n7149[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7178 = n7176 | n7177;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7179 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7180 = ~n7179;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7181 = n7178 & n7180;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7182 = inhibit[4]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7183 = ~n7182;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7184 = n7181 & n7183;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7186 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7187 = hpmevent[296:286]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7188 = n7186 & n7187;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7194 = n7188[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7196 = 1'b0 | n7194;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7198 = n7188[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7199 = n7196 | n7198;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7200 = n7188[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7201 = n7199 | n7200;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7202 = n7188[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7203 = n7201 | n7202;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7204 = n7188[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7205 = n7203 | n7204;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7206 = n7188[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7207 = n7205 | n7206;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7208 = n7188[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7209 = n7207 | n7208;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7210 = n7188[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7211 = n7209 | n7210;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7212 = n7188[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7213 = n7211 | n7212;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7214 = n7188[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7215 = n7213 | n7214;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7216 = n7188[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7217 = n7215 | n7216;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7218 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7219 = ~n7218;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7220 = n7217 & n7219;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7221 = inhibit[5]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7222 = ~n7221;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7223 = n7220 & n7222;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7225 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7226 = hpmevent[285:275]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7227 = n7225 & n7226;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7233 = n7227[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7235 = 1'b0 | n7233;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7237 = n7227[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7238 = n7235 | n7237;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7239 = n7227[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7240 = n7238 | n7239;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7241 = n7227[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7242 = n7240 | n7241;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7243 = n7227[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7244 = n7242 | n7243;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7245 = n7227[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7246 = n7244 | n7245;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7247 = n7227[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7248 = n7246 | n7247;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7249 = n7227[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7250 = n7248 | n7249;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7251 = n7227[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7252 = n7250 | n7251;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7253 = n7227[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7254 = n7252 | n7253;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7255 = n7227[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7256 = n7254 | n7255;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7257 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7258 = ~n7257;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7259 = n7256 & n7258;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7260 = inhibit[6]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7261 = ~n7260;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7262 = n7259 & n7261;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7264 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7265 = hpmevent[274:264]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7266 = n7264 & n7265;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7272 = n7266[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7274 = 1'b0 | n7272;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7276 = n7266[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7277 = n7274 | n7276;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7278 = n7266[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7279 = n7277 | n7278;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7280 = n7266[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7281 = n7279 | n7280;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7282 = n7266[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7283 = n7281 | n7282;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7284 = n7266[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7285 = n7283 | n7284;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7286 = n7266[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7287 = n7285 | n7286;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7288 = n7266[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7289 = n7287 | n7288;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7290 = n7266[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7291 = n7289 | n7290;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7292 = n7266[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7293 = n7291 | n7292;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7294 = n7266[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7295 = n7293 | n7294;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7296 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7297 = ~n7296;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7298 = n7295 & n7297;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7299 = inhibit[7]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7300 = ~n7299;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7301 = n7298 & n7300;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7303 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7304 = hpmevent[263:253]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7305 = n7303 & n7304;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7311 = n7305[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7313 = 1'b0 | n7311;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7315 = n7305[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7316 = n7313 | n7315;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7317 = n7305[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7318 = n7316 | n7317;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7319 = n7305[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7320 = n7318 | n7319;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7321 = n7305[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7322 = n7320 | n7321;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7323 = n7305[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7324 = n7322 | n7323;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7325 = n7305[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7326 = n7324 | n7325;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7327 = n7305[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7328 = n7326 | n7327;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7329 = n7305[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7330 = n7328 | n7329;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7331 = n7305[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7332 = n7330 | n7331;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7333 = n7305[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7334 = n7332 | n7333;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7335 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7336 = ~n7335;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7337 = n7334 & n7336;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7338 = inhibit[8]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7339 = ~n7338;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7340 = n7337 & n7339;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7342 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7343 = hpmevent[252:242]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7344 = n7342 & n7343;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7350 = n7344[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7352 = 1'b0 | n7350;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7354 = n7344[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7355 = n7352 | n7354;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7356 = n7344[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7357 = n7355 | n7356;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7358 = n7344[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7359 = n7357 | n7358;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7360 = n7344[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7361 = n7359 | n7360;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7362 = n7344[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7363 = n7361 | n7362;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7364 = n7344[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7365 = n7363 | n7364;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7366 = n7344[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7367 = n7365 | n7366;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7368 = n7344[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7369 = n7367 | n7368;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7370 = n7344[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7371 = n7369 | n7370;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7372 = n7344[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7373 = n7371 | n7372;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7374 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7375 = ~n7374;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7376 = n7373 & n7375;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7377 = inhibit[9]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7378 = ~n7377;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7379 = n7376 & n7378;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7381 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7382 = hpmevent[241:231]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7383 = n7381 & n7382;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7389 = n7383[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7391 = 1'b0 | n7389;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7393 = n7383[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7394 = n7391 | n7393;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7395 = n7383[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7396 = n7394 | n7395;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7397 = n7383[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7398 = n7396 | n7397;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7399 = n7383[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7400 = n7398 | n7399;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7401 = n7383[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7402 = n7400 | n7401;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7403 = n7383[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7404 = n7402 | n7403;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7405 = n7383[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7406 = n7404 | n7405;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7407 = n7383[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7408 = n7406 | n7407;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7409 = n7383[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7410 = n7408 | n7409;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7411 = n7383[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7412 = n7410 | n7411;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7413 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7414 = ~n7413;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7415 = n7412 & n7414;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7416 = inhibit[10]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7417 = ~n7416;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7418 = n7415 & n7417;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7420 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7421 = hpmevent[230:220]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7422 = n7420 & n7421;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7428 = n7422[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7430 = 1'b0 | n7428;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7432 = n7422[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7433 = n7430 | n7432;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7434 = n7422[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7435 = n7433 | n7434;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7436 = n7422[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7437 = n7435 | n7436;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7438 = n7422[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7439 = n7437 | n7438;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7440 = n7422[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7441 = n7439 | n7440;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7442 = n7422[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7443 = n7441 | n7442;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7444 = n7422[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7445 = n7443 | n7444;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7446 = n7422[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7447 = n7445 | n7446;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7448 = n7422[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7449 = n7447 | n7448;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7450 = n7422[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7451 = n7449 | n7450;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7452 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7453 = ~n7452;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7454 = n7451 & n7453;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7455 = inhibit[11]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7456 = ~n7455;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7457 = n7454 & n7456;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7459 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7460 = hpmevent[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7461 = n7459 & n7460;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7467 = n7461[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7469 = 1'b0 | n7467;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7471 = n7461[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7472 = n7469 | n7471;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7473 = n7461[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7474 = n7472 | n7473;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7475 = n7461[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7476 = n7474 | n7475;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7477 = n7461[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7478 = n7476 | n7477;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7479 = n7461[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7480 = n7478 | n7479;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7481 = n7461[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7482 = n7480 | n7481;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7483 = n7461[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7484 = n7482 | n7483;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7485 = n7461[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7486 = n7484 | n7485;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7487 = n7461[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7488 = n7486 | n7487;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7489 = n7461[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7490 = n7488 | n7489;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7491 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7492 = ~n7491;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7493 = n7490 & n7492;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7494 = inhibit[12]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7495 = ~n7494;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7496 = n7493 & n7495;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7498 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7499 = hpmevent[208:198]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7500 = n7498 & n7499;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7506 = n7500[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7508 = 1'b0 | n7506;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7510 = n7500[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7511 = n7508 | n7510;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7512 = n7500[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7513 = n7511 | n7512;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7514 = n7500[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7515 = n7513 | n7514;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7516 = n7500[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7517 = n7515 | n7516;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7518 = n7500[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7519 = n7517 | n7518;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7520 = n7500[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7521 = n7519 | n7520;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7522 = n7500[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7523 = n7521 | n7522;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7524 = n7500[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7525 = n7523 | n7524;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7526 = n7500[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7527 = n7525 | n7526;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7528 = n7500[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7529 = n7527 | n7528;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7530 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7531 = ~n7530;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7532 = n7529 & n7531;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7533 = inhibit[13]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7534 = ~n7533;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7535 = n7532 & n7534;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7537 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7538 = hpmevent[197:187]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7539 = n7537 & n7538;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7545 = n7539[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7547 = 1'b0 | n7545;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7549 = n7539[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7550 = n7547 | n7549;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7551 = n7539[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7552 = n7550 | n7551;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7553 = n7539[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7554 = n7552 | n7553;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7555 = n7539[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7556 = n7554 | n7555;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7557 = n7539[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7558 = n7556 | n7557;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7559 = n7539[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7560 = n7558 | n7559;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7561 = n7539[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7562 = n7560 | n7561;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7563 = n7539[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7564 = n7562 | n7563;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7565 = n7539[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7566 = n7564 | n7565;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7567 = n7539[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7568 = n7566 | n7567;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7569 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7570 = ~n7569;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7571 = n7568 & n7570;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7572 = inhibit[14]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7573 = ~n7572;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7574 = n7571 & n7573;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7576 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7577 = hpmevent[186:176]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7578 = n7576 & n7577;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7584 = n7578[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7586 = 1'b0 | n7584;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7588 = n7578[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7589 = n7586 | n7588;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7590 = n7578[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7591 = n7589 | n7590;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7592 = n7578[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7593 = n7591 | n7592;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7594 = n7578[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7595 = n7593 | n7594;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7596 = n7578[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7597 = n7595 | n7596;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7598 = n7578[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7599 = n7597 | n7598;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7600 = n7578[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7601 = n7599 | n7600;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7602 = n7578[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7603 = n7601 | n7602;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7604 = n7578[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7605 = n7603 | n7604;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7606 = n7578[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7607 = n7605 | n7606;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7608 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7609 = ~n7608;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7610 = n7607 & n7609;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7611 = inhibit[15]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7612 = ~n7611;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7613 = n7610 & n7612;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7615 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7616 = hpmevent[175:165]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7617 = n7615 & n7616;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7623 = n7617[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7625 = 1'b0 | n7623;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7627 = n7617[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7628 = n7625 | n7627;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7629 = n7617[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7630 = n7628 | n7629;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7631 = n7617[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7632 = n7630 | n7631;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7633 = n7617[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7634 = n7632 | n7633;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7635 = n7617[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7636 = n7634 | n7635;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7637 = n7617[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7638 = n7636 | n7637;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7639 = n7617[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7640 = n7638 | n7639;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7641 = n7617[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7642 = n7640 | n7641;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7643 = n7617[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7644 = n7642 | n7643;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7645 = n7617[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7646 = n7644 | n7645;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7647 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7648 = ~n7647;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7649 = n7646 & n7648;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7650 = inhibit[16]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7651 = ~n7650;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7652 = n7649 & n7651;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7654 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7655 = hpmevent[164:154]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7656 = n7654 & n7655;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7662 = n7656[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7664 = 1'b0 | n7662;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7666 = n7656[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7667 = n7664 | n7666;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7668 = n7656[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7669 = n7667 | n7668;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7670 = n7656[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7671 = n7669 | n7670;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7672 = n7656[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7673 = n7671 | n7672;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7674 = n7656[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7675 = n7673 | n7674;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7676 = n7656[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7677 = n7675 | n7676;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7678 = n7656[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7679 = n7677 | n7678;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7680 = n7656[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7681 = n7679 | n7680;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7682 = n7656[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7683 = n7681 | n7682;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7684 = n7656[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7685 = n7683 | n7684;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7686 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7687 = ~n7686;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7688 = n7685 & n7687;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7689 = inhibit[17]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7690 = ~n7689;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7691 = n7688 & n7690;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7693 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7694 = hpmevent[153:143]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7695 = n7693 & n7694;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7701 = n7695[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7703 = 1'b0 | n7701;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7705 = n7695[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7706 = n7703 | n7705;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7707 = n7695[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7708 = n7706 | n7707;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7709 = n7695[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7710 = n7708 | n7709;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7711 = n7695[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7712 = n7710 | n7711;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7713 = n7695[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7714 = n7712 | n7713;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7715 = n7695[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7716 = n7714 | n7715;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7717 = n7695[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7718 = n7716 | n7717;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7719 = n7695[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7720 = n7718 | n7719;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7721 = n7695[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7722 = n7720 | n7721;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7723 = n7695[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7724 = n7722 | n7723;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7725 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7726 = ~n7725;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7727 = n7724 & n7726;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7728 = inhibit[18]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7729 = ~n7728;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7730 = n7727 & n7729;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7732 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7733 = hpmevent[142:132]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7734 = n7732 & n7733;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7740 = n7734[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7742 = 1'b0 | n7740;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7744 = n7734[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7745 = n7742 | n7744;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7746 = n7734[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7747 = n7745 | n7746;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7748 = n7734[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7749 = n7747 | n7748;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7750 = n7734[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7751 = n7749 | n7750;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7752 = n7734[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7753 = n7751 | n7752;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7754 = n7734[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7755 = n7753 | n7754;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7756 = n7734[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7757 = n7755 | n7756;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7758 = n7734[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7759 = n7757 | n7758;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7760 = n7734[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7761 = n7759 | n7760;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7762 = n7734[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7763 = n7761 | n7762;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7764 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7765 = ~n7764;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7766 = n7763 & n7765;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7767 = inhibit[19]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7768 = ~n7767;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7769 = n7766 & n7768;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7771 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7772 = hpmevent[131:121]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7773 = n7771 & n7772;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7779 = n7773[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7781 = 1'b0 | n7779;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7783 = n7773[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7784 = n7781 | n7783;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7785 = n7773[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7786 = n7784 | n7785;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7787 = n7773[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7788 = n7786 | n7787;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7789 = n7773[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7790 = n7788 | n7789;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7791 = n7773[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7792 = n7790 | n7791;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7793 = n7773[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7794 = n7792 | n7793;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7795 = n7773[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7796 = n7794 | n7795;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7797 = n7773[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7798 = n7796 | n7797;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7799 = n7773[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7800 = n7798 | n7799;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7801 = n7773[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7802 = n7800 | n7801;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7803 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7804 = ~n7803;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7805 = n7802 & n7804;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7806 = inhibit[20]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7807 = ~n7806;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7808 = n7805 & n7807;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7810 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7811 = hpmevent[120:110]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7812 = n7810 & n7811;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7818 = n7812[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7820 = 1'b0 | n7818;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7822 = n7812[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7823 = n7820 | n7822;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7824 = n7812[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7825 = n7823 | n7824;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7826 = n7812[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7827 = n7825 | n7826;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7828 = n7812[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7829 = n7827 | n7828;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7830 = n7812[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7831 = n7829 | n7830;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7832 = n7812[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7833 = n7831 | n7832;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7834 = n7812[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7835 = n7833 | n7834;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7836 = n7812[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7837 = n7835 | n7836;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7838 = n7812[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7839 = n7837 | n7838;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7840 = n7812[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7841 = n7839 | n7840;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7842 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7843 = ~n7842;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7844 = n7841 & n7843;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7845 = inhibit[21]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7846 = ~n7845;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7847 = n7844 & n7846;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7849 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7850 = hpmevent[109:99]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7851 = n7849 & n7850;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7857 = n7851[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7859 = 1'b0 | n7857;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7861 = n7851[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7862 = n7859 | n7861;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7863 = n7851[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7864 = n7862 | n7863;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7865 = n7851[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7866 = n7864 | n7865;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7867 = n7851[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7868 = n7866 | n7867;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7869 = n7851[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7870 = n7868 | n7869;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7871 = n7851[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7872 = n7870 | n7871;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7873 = n7851[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7874 = n7872 | n7873;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7875 = n7851[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7876 = n7874 | n7875;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7877 = n7851[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7878 = n7876 | n7877;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7879 = n7851[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7880 = n7878 | n7879;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7881 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7882 = ~n7881;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7883 = n7880 & n7882;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7884 = inhibit[22]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7885 = ~n7884;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7886 = n7883 & n7885;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7888 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7889 = hpmevent[98:88]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7890 = n7888 & n7889;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7896 = n7890[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7898 = 1'b0 | n7896;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7900 = n7890[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7901 = n7898 | n7900;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7902 = n7890[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7903 = n7901 | n7902;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7904 = n7890[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7905 = n7903 | n7904;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7906 = n7890[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7907 = n7905 | n7906;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7908 = n7890[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7909 = n7907 | n7908;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7910 = n7890[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7911 = n7909 | n7910;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7912 = n7890[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7913 = n7911 | n7912;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7914 = n7890[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7915 = n7913 | n7914;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7916 = n7890[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7917 = n7915 | n7916;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7918 = n7890[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7919 = n7917 | n7918;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7920 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7921 = ~n7920;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7922 = n7919 & n7921;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7923 = inhibit[23]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7924 = ~n7923;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7925 = n7922 & n7924;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7927 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7928 = hpmevent[87:77]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7929 = n7927 & n7928;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7935 = n7929[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7937 = 1'b0 | n7935;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7939 = n7929[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7940 = n7937 | n7939;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7941 = n7929[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7942 = n7940 | n7941;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7943 = n7929[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7944 = n7942 | n7943;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7945 = n7929[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7946 = n7944 | n7945;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7947 = n7929[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7948 = n7946 | n7947;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7949 = n7929[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7950 = n7948 | n7949;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7951 = n7929[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7952 = n7950 | n7951;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7953 = n7929[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7954 = n7952 | n7953;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7955 = n7929[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7956 = n7954 | n7955;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7957 = n7929[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7958 = n7956 | n7957;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7959 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7960 = ~n7959;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n7961 = n7958 & n7960;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n7962 = inhibit[24]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n7963 = ~n7962;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n7964 = n7961 & n7963;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n7966 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n7967 = hpmevent[76:66]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n7968 = n7966 & n7967;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7974 = n7968[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7976 = 1'b0 | n7974;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7978 = n7968[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7979 = n7976 | n7978;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7980 = n7968[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7981 = n7979 | n7980;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7982 = n7968[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7983 = n7981 | n7982;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7984 = n7968[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7985 = n7983 | n7984;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7986 = n7968[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7987 = n7985 | n7986;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7988 = n7968[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7989 = n7987 | n7988;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7990 = n7968[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7991 = n7989 | n7990;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7992 = n7968[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7993 = n7991 | n7992;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7994 = n7968[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7995 = n7993 | n7994;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n7996 = n7968[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n7997 = n7995 | n7996;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n7998 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n7999 = ~n7998;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n8000 = n7997 & n7999;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n8001 = inhibit[25]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n8002 = ~n8001;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n8003 = n8000 & n8002;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n8005 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n8006 = hpmevent[65:55]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n8007 = n8005 & n8006;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8013 = n8007[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8015 = 1'b0 | n8013;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8017 = n8007[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8018 = n8015 | n8017;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8019 = n8007[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8020 = n8018 | n8019;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8021 = n8007[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8022 = n8020 | n8021;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8023 = n8007[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8024 = n8022 | n8023;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8025 = n8007[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8026 = n8024 | n8025;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8027 = n8007[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8028 = n8026 | n8027;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8029 = n8007[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8030 = n8028 | n8029;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8031 = n8007[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8032 = n8030 | n8031;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8033 = n8007[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8034 = n8032 | n8033;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8035 = n8007[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8036 = n8034 | n8035;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n8037 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n8038 = ~n8037;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n8039 = n8036 & n8038;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n8040 = inhibit[26]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n8041 = ~n8040;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n8042 = n8039 & n8041;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n8044 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n8045 = hpmevent[54:44]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n8046 = n8044 & n8045;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8052 = n8046[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8054 = 1'b0 | n8052;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8056 = n8046[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8057 = n8054 | n8056;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8058 = n8046[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8059 = n8057 | n8058;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8060 = n8046[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8061 = n8059 | n8060;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8062 = n8046[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8063 = n8061 | n8062;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8064 = n8046[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8065 = n8063 | n8064;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8066 = n8046[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8067 = n8065 | n8066;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8068 = n8046[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8069 = n8067 | n8068;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8070 = n8046[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8071 = n8069 | n8070;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8072 = n8046[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8073 = n8071 | n8072;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8074 = n8046[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8075 = n8073 | n8074;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n8076 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n8077 = ~n8076;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n8078 = n8075 & n8077;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n8079 = inhibit[27]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n8080 = ~n8079;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n8081 = n8078 & n8080;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n8083 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n8084 = hpmevent[43:33]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n8085 = n8083 & n8084;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8091 = n8085[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8093 = 1'b0 | n8091;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8095 = n8085[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8096 = n8093 | n8095;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8097 = n8085[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8098 = n8096 | n8097;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8099 = n8085[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8100 = n8098 | n8099;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8101 = n8085[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8102 = n8100 | n8101;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8103 = n8085[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8104 = n8102 | n8103;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8105 = n8085[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8106 = n8104 | n8105;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8107 = n8085[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8108 = n8106 | n8107;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8109 = n8085[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8110 = n8108 | n8109;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8111 = n8085[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8112 = n8110 | n8111;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8113 = n8085[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8114 = n8112 | n8113;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n8115 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n8116 = ~n8115;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n8117 = n8114 & n8116;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n8118 = inhibit[28]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n8119 = ~n8118;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n8120 = n8117 & n8119;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n8122 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n8123 = hpmevent[32:22]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n8124 = n8122 & n8123;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8130 = n8124[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8132 = 1'b0 | n8130;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8134 = n8124[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8135 = n8132 | n8134;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8136 = n8124[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8137 = n8135 | n8136;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8138 = n8124[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8139 = n8137 | n8138;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8140 = n8124[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8141 = n8139 | n8140;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8142 = n8124[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8143 = n8141 | n8142;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8144 = n8124[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8145 = n8143 | n8144;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8146 = n8124[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8147 = n8145 | n8146;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8148 = n8124[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8149 = n8147 | n8148;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8150 = n8124[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8151 = n8149 | n8150;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8152 = n8124[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8153 = n8151 | n8152;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n8154 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n8155 = ~n8154;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n8156 = n8153 & n8155;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n8157 = inhibit[29]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n8158 = ~n8157;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n8159 = n8156 & n8158;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n8161 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n8162 = hpmevent[21:11]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n8163 = n8161 & n8162;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8169 = n8163[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8171 = 1'b0 | n8169;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8173 = n8163[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8174 = n8171 | n8173;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8175 = n8163[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8176 = n8174 | n8175;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8177 = n8163[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8178 = n8176 | n8177;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8179 = n8163[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8180 = n8178 | n8179;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8181 = n8163[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8182 = n8180 | n8181;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8183 = n8163[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8184 = n8182 | n8183;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8185 = n8163[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8186 = n8184 | n8185;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8187 = n8163[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8188 = n8186 | n8187;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8189 = n8163[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8190 = n8188 | n8189;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8191 = n8163[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8192 = n8190 | n8191;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n8193 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n8194 = ~n8193;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n8195 = n8192 & n8194;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n8196 = inhibit[30]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n8197 = ~n8196;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n8198 = n8195 & n8197;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:38  */
  assign n8200 = n6020[219:209]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:60  */
  assign n8201 = hpmevent[10:0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:48  */
  assign n8202 = n8200 & n8201;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8208 = n8202[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8210 = 1'b0 | n8208;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8212 = n8202[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8213 = n8210 | n8212;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8214 = n8202[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8215 = n8213 | n8214;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8216 = n8202[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8217 = n8215 | n8216;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8218 = n8202[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8219 = n8217 | n8218;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8220 = n8202[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8221 = n8219 | n8220;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8222 = n8202[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8223 = n8221 | n8222;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8224 = n8202[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8225 = n8223 | n8224;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8226 = n8202[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8227 = n8225 | n8226;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8228 = n8202[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8229 = n8227 | n8228;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n8230 = n8202[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n8231 = n8229 | n8230;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:81  */
  assign n8232 = n6020[261]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:70  */
  assign n8233 = ~n8232;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:65  */
  assign n8234 = n8231 & n8233;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:108  */
  assign n8235 = inhibit[31]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:97  */
  assign n8236 = ~n8235;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:189:92  */
  assign n8237 = n8234 & n8236;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:199:5  */
  neorv32_prim_cnt_64 base_enabled_cycle_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .inc_i(n8238),
    .we_i(n8239),
    .data_i(n8240),
    .oe_i(n8241),
    .cnt_o(cycle_rd));
  /* ../../rtl/core/neorv32_cpu_counters.vhd:206:24  */
  assign n8238 = cnt_inc[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:207:23  */
  assign n8239 = cnt_we[63:62]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:208:24  */
  assign n8240 = n6020[208:177]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:209:23  */
  assign n8241 = cnt_re[0]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:216:18  */
  assign n8244 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:222:35  */
  assign n8250 = cnt_re[1]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:222:23  */
  assign n8251 = n8250 ? time_q : 64'b0000000000000000000000000000000000000000000000000000000000000000;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:225:5  */
  neorv32_prim_cnt_64 base_enabled_instret_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .inc_i(n8253),
    .we_i(n8254),
    .data_i(n8255),
    .oe_i(n8256),
    .cnt_o(instret_rd));
  /* ../../rtl/core/neorv32_cpu_counters.vhd:232:24  */
  assign n8253 = cnt_inc[2]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:233:23  */
  assign n8254 = cnt_we[59:58]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:234:24  */
  assign n8255 = n6020[208:177]; // extract
  /* ../../rtl/core/neorv32_cpu_counters.vhd:235:23  */
  assign n8256 = cnt_re[2]; // extract
  assign n8262 = {n6040, n6034, n6071, n6065, n6102, n6096, n6133, n6127, n6164, n6158, n6195, n6189, n6226, n6220, n6257, n6251, n6288, n6282, n6319, n6313, n6350, n6344, n6381, n6375, n6412, n6406, n6443, n6437, n6474, n6468, n6505, n6499, n6536, n6530, n6567, n6561, n6598, n6592, n6629, n6623, n6660, n6654, n6691, n6685, n6722, n6716, n6753, n6747, n6784, n6778, n6815, n6809, n6846, n6840, n6877, n6871, n6908, n6902, n6939, n6933, n6970, n6964, n7001, n6995};
  assign n8263 = {n6987, n6956, n6925, n6894, n6863, n6832, n6801, n6770, n6739, n6708, n6677, n6646, n6615, n6584, n6553, n6522, n6491, n6460, n6429, n6398, n6367, n6336, n6305, n6274, n6243, n6212, n6181, n6150, n6119, n6088, n6057, n6026};
  assign n8264 = {n7005, n6974, n6943, n6912, n6881, n6850, n6819, n6788, n6757, n6726, n6695, n6664, n6633, n6602, n6571, n6540, n6509, n6478, n6447, n6416, n6385, n6354, n6323, n6292, n6261, n6230, n6199, n6168, n6137, n6106, n6075, n6044};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:99:5  */
  always @(posedge clk_i or posedge n7060)
    if (n7060)
      n8267 <= 32'b00000000000000000000000000000000;
    else
      n8267 <= n7072;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:97:5  */
  assign n8268 = {n8237, n8198, n8159, n8120, n8081, n8042, n8003, n7964, n7925, n7886, n7847, n7808, n7769, n7730, n7691, n7652, n7613, n7574, n7535, n7496, n7457, n7418, n7379, n7340, n7301, n7262, n7223, n7184, n7145, n7106, 1'b0, n7095};
  /* ../../rtl/core/neorv32_cpu_counters.vhd:218:7  */
  always @(posedge clk_i or posedge n8244)
    if (n8244)
      n8269 <= 64'b0000000000000000000000000000000000000000000000000000000000000000;
    else
      n8269 <= mtime_i;
  /* ../../rtl/core/neorv32_cpu_counters.vhd:216:7  */
  assign n8270 = {32'b00000000000000000000000000000000, n7079};
endmodule

module neorv32_cpu_control_0_e740762aedb36b110599737dc13103f0a38aaf3e
  (input  clk_i,
   input  rstn_i,
   input  \frontend_i_frontend_i[valid] ,
   input  [31:0] \frontend_i_frontend_i[i32] ,
   input  [15:0] \frontend_i_frontend_i[i16] ,
   input  \frontend_i_frontend_i[compr] ,
   input  \frontend_i_frontend_i[fault] ,
   input  hwtrig_i,
   input  alu_cp_done_i,
   input  [1:0] alu_cmp_i,
   input  [31:0] alu_add_i,
   input  [31:0] rf_rs1_i,
   input  [31:0] xcsr_rdata_i,
   input  irq_dbg_i,
   input  [2:0] irq_machine_i,
   input  [15:0] irq_fast_i,
   input  lsu_wait_i,
   input  [31:0] lsu_mar_i,
   input  [3:0] lsu_err_i,
   output \ctrl_o_ctrl_o[if_reset] ,
   output \ctrl_o_ctrl_o[if_ready] ,
   output [31:0] \ctrl_o_ctrl_o[pc_cur] ,
   output [31:0] \ctrl_o_ctrl_o[pc_nxt] ,
   output [31:0] \ctrl_o_ctrl_o[pc_ret] ,
   output \ctrl_o_ctrl_o[rf_wb_en] ,
   output [4:0] \ctrl_o_ctrl_o[rf_rs1] ,
   output [4:0] \ctrl_o_ctrl_o[rf_rs2] ,
   output [4:0] \ctrl_o_ctrl_o[rf_rd] ,
   output \ctrl_o_ctrl_o[rf_zero] ,
   output [2:0] \ctrl_o_ctrl_o[alu_op] ,
   output \ctrl_o_ctrl_o[alu_sub] ,
   output \ctrl_o_ctrl_o[alu_opa_mux] ,
   output \ctrl_o_ctrl_o[alu_opb_mux] ,
   output \ctrl_o_ctrl_o[alu_unsigned] ,
   output [31:0] \ctrl_o_ctrl_o[alu_imm] ,
   output \ctrl_o_ctrl_o[alu_cp_alu] ,
   output \ctrl_o_ctrl_o[alu_cp_cfu] ,
   output \ctrl_o_ctrl_o[alu_cp_fpu] ,
   output \ctrl_o_ctrl_o[lsu_req] ,
   output \ctrl_o_ctrl_o[lsu_rd] ,
   output \ctrl_o_ctrl_o[lsu_wr] ,
   output \ctrl_o_ctrl_o[lsu_mo_en] ,
   output \ctrl_o_ctrl_o[lsu_mi_en] ,
   output \ctrl_o_ctrl_o[lsu_priv] ,
   output \ctrl_o_ctrl_o[csr_we] ,
   output \ctrl_o_ctrl_o[csr_re] ,
   output [11:0] \ctrl_o_ctrl_o[csr_addr] ,
   output [31:0] \ctrl_o_ctrl_o[csr_wdata] ,
   output [10:0] \ctrl_o_ctrl_o[cnt_event] ,
   output [2:0] \ctrl_o_ctrl_o[ir_funct3] ,
   output [11:0] \ctrl_o_ctrl_o[ir_funct12] ,
   output [6:0] \ctrl_o_ctrl_o[ir_opcode] ,
   output [15:0] \ctrl_o_ctrl_o[ir_rvc] ,
   output \ctrl_o_ctrl_o[cpu_priv] ,
   output \ctrl_o_ctrl_o[cpu_trap] ,
   output \ctrl_o_ctrl_o[cpu_sync_exc] ,
   output \ctrl_o_ctrl_o[cpu_debug] ,
   output [1:0] \ctrl_o_ctrl_o[cpu_fence] ,
   output [31:0] csr_rdata_o);
  wire n2718;
  wire n2719;
  wire [31:0] n2720;
  wire [31:0] n2721;
  wire [31:0] n2722;
  wire n2723;
  wire [4:0] n2724;
  wire [4:0] n2725;
  wire [4:0] n2726;
  wire n2727;
  wire [2:0] n2728;
  wire n2729;
  wire n2730;
  wire n2731;
  wire n2732;
  wire [31:0] n2733;
  wire n2734;
  wire n2735;
  wire n2736;
  wire n2737;
  wire n2738;
  wire n2739;
  wire n2740;
  wire n2741;
  wire n2742;
  wire n2743;
  wire n2744;
  wire [11:0] n2745;
  wire [31:0] n2746;
  wire [10:0] n2747;
  wire [2:0] n2748;
  wire [11:0] n2749;
  wire [6:0] n2750;
  wire [15:0] n2751;
  wire n2752;
  wire n2753;
  wire n2754;
  wire n2755;
  wire [1:0] n2756;
  wire [50:0] n2757;
  wire [116:0] exec;
  wire [116:0] exec_nxt;
  wire [263:0] ctrl;
  wire [263:0] ctrl_nxt;
  wire [101:0] trap;
  wire [261:0] csr;
  wire [31:0] csr_wdata;
  wire [31:0] csr_rdata;
  wire [4:0] debug_ctrl;
  wire branch_taken;
  wire [9:0] monitor_cnt;
  wire [2:0] csr_valid;
  wire illegal_cmd;
  wire [10:0] cnt_event;
  wire ebreak_trig;
  wire [6:0] trap_env;
  wire n2760;
  wire n2761;
  wire n2762;
  wire n2763;
  wire n2764;
  wire n2765;
  wire n2766;
  wire n2767;
  wire n2768;
  wire n2769;
  wire n2770;
  wire n2772;
  wire n2775;
  wire [116:0] n2785;
  wire [4:0] n2794;
  wire [6:0] n2796;
  wire [6:0] n2797;
  wire [2:0] n2798;
  wire [11:0] n2805;
  localparam [263:0] n2806 = 264'b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;
  wire n2809;
  wire n2812;
  wire n2815;
  wire [3:0] n2821;
  wire [3:0] n2822;
  wire [3:0] n2823;
  wire [3:0] n2824;
  wire [3:0] n2825;
  wire [15:0] n2826;
  wire [4:0] n2827;
  wire [20:0] n2828;
  wire [5:0] n2830;
  wire [26:0] n2831;
  wire [4:0] n2832;
  wire [31:0] n2833;
  wire n2835;
  wire n2837;
  wire [3:0] n2843;
  wire [3:0] n2844;
  wire [3:0] n2845;
  wire [3:0] n2846;
  wire [3:0] n2847;
  wire [15:0] n2848;
  wire [19:0] n2849;
  wire n2851;
  wire [20:0] n2852;
  wire [5:0] n2853;
  wire [26:0] n2854;
  wire [3:0] n2855;
  wire [30:0] n2856;
  wire [31:0] n2858;
  wire n2860;
  wire [19:0] n2861;
  wire [31:0] n2863;
  wire n2865;
  wire n2867;
  wire n2868;
  wire n2870;
  wire [3:0] n2876;
  wire [3:0] n2877;
  wire [3:0] n2878;
  wire [11:0] n2879;
  wire [7:0] n2881;
  wire [19:0] n2882;
  wire n2883;
  wire [20:0] n2884;
  wire [9:0] n2885;
  wire [30:0] n2886;
  wire [31:0] n2888;
  wire n2890;
  wire n2893;
  wire n2895;
  wire [3:0] n2901;
  wire [3:0] n2902;
  wire [3:0] n2903;
  wire [3:0] n2904;
  wire [3:0] n2905;
  wire [15:0] n2906;
  wire [4:0] n2907;
  wire [20:0] n2908;
  wire [9:0] n2910;
  wire [30:0] n2911;
  wire n2912;
  wire [31:0] n2913;
  wire [4:0] n2914;
  reg [31:0] n2915;
  wire n2918;
  wire n2919;
  wire n2920;
  wire n2921;
  wire n2924;
  wire n2926;
  wire n2927;
  wire n2929;
  wire n2930;
  wire n2932;
  wire n2933;
  wire n2934;
  wire n2937;
  wire n2939;
  wire [3:0] n2940;
  wire n2945;
  wire n2948;
  wire n2950;
  wire [31:0] n2953;
  wire n2954;
  wire n2955;
  wire n2956;
  wire n2958;
  wire n2959;
  wire n2960;
  wire n2961;
  wire n2962;
  wire [31:0] n2963;
  wire [15:0] n2964;
  wire [30:0] n2965;
  wire [31:0] n2967;
  wire [4:0] n2969;
  wire n2971;
  wire [11:0] n2972;
  wire [11:0] n2973;
  wire [84:0] n2974;
  wire [84:0] n2975;
  wire [84:0] n2976;
  wire n2977;
  wire n2978;
  wire [3:0] n2979;
  wire [3:0] n2980;
  wire [80:0] n2981;
  wire [80:0] n2982;
  wire [80:0] n2983;
  wire [11:0] n2984;
  wire n2985;
  wire n2987;
  wire n2990;
  wire n2991;
  wire n2992;
  wire [24:0] n2993;
  wire [4:0] n2994;
  wire [29:0] n2995;
  wire [31:0] n2997;
  wire [29:0] n2998;
  wire [31:0] n3000;
  wire [31:0] n3001;
  wire n3005;
  wire [30:0] n3007;
  wire [31:0] n3009;
  wire n3013;
  wire [30:0] n3014;
  wire [31:0] n3016;
  wire n3019;
  wire n3022;
  wire n3024;
  wire n3027;
  wire n3030;
  wire n3033;
  wire [5:0] n3035;
  reg [2:0] n3036;
  wire [1:0] n3037;
  wire n3039;
  wire n3041;
  wire n3042;
  wire n3043;
  wire n3044;
  wire n3045;
  wire n3046;
  wire n3048;
  wire n3049;
  wire n3050;
  wire n3051;
  wire n3053;
  wire n3054;
  wire n3056;
  wire n3057;
  wire n3058;
  wire n3060;
  wire n3062;
  wire n3063;
  wire n3065;
  wire n3067;
  wire n3068;
  wire n3069;
  wire n3071;
  wire n3073;
  wire n3074;
  wire n3075;
  wire n3077;
  wire n3079;
  wire n3080;
  wire n3081;
  wire n3083;
  wire n3085;
  wire n3086;
  wire n3087;
  wire n3089;
  wire n3091;
  wire n3092;
  wire n3093;
  wire n3095;
  wire n3097;
  wire n3098;
  wire n3099;
  wire n3100;
  wire n3101;
  wire [3:0] n3106;
  wire n3107;
  wire n3108;
  wire n3109;
  wire n3110;
  wire n3112;
  wire n3114;
  wire n3115;
  wire n3120;
  wire n3124;
  wire n3125;
  wire n3126;
  wire n3127;
  wire n3130;
  wire n3132;
  wire n3133;
  wire n3135;
  wire n3136;
  wire n3139;
  wire n3141;
  wire n3142;
  wire n3144;
  wire n3145;
  wire n3146;
  wire [1:0] n3148;
  wire n3151;
  wire n3155;
  wire n3159;
  wire n3161;
  wire n3162;
  wire n3164;
  wire n3165;
  wire n3167;
  wire n3168;
  wire n3170;
  wire n3172;
  wire n3173;
  wire n3175;
  wire n3176;
  wire [7:0] n3178;
  reg [3:0] n3179;
  wire n3180;
  reg n3181;
  wire [2:0] n3182;
  reg [2:0] n3183;
  wire n3184;
  reg n3185;
  wire n3186;
  reg n3187;
  wire n3188;
  reg n3189;
  wire n3190;
  reg n3191;
  reg n3192;
  reg n3193;
  wire n3194;
  reg n3195;
  wire [1:0] n3196;
  reg [1:0] n3197;
  wire n3199;
  wire n3208;
  wire n3210;
  wire n3212;
  wire n3213;
  wire n3214;
  wire n3215;
  wire n3216;
  wire [3:0] n3218;
  wire [3:0] n3219;
  wire n3221;
  wire n3223;
  wire n3225;
  wire n3226;
  wire n3227;
  wire n3230;
  wire [30:0] n3231;
  wire [31:0] n3233;
  wire [31:0] n3234;
  wire [31:0] n3235;
  wire n3236;
  wire [30:0] n3237;
  wire [31:0] n3239;
  wire n3240;
  wire n3243;
  wire n3251;
  wire n3253;
  wire n3255;
  wire n3256;
  wire n3257;
  wire n3258;
  wire n3259;
  wire [3:0] n3263;
  wire n3264;
  wire n3265;
  wire n3267;
  wire n3268;
  wire n3276;
  wire n3278;
  wire n3280;
  wire n3281;
  wire n3282;
  wire n3283;
  wire n3284;
  wire n3285;
  wire n3286;
  wire n3287;
  wire [3:0] n3289;
  wire [3:0] n3290;
  wire n3291;
  wire n3292;
  wire n3294;
  wire n3303;
  wire n3305;
  wire n3307;
  wire n3308;
  wire n3309;
  wire n3310;
  wire n3311;
  wire n3313;
  wire [2:0] n3314;
  wire n3317;
  wire n3320;
  wire n3323;
  wire n3326;
  wire [3:0] n3328;
  reg [3:0] n3329;
  reg n3330;
  reg n3331;
  wire n3333;
  wire n3335;
  wire n3337;
  wire n3338;
  wire [4:0] n3339;
  wire n3341;
  wire n3342;
  wire n3343;
  wire n3345;
  wire n3346;
  wire [3:0] n3347;
  wire n3348;
  wire n3349;
  wire [1:0] n3350;
  wire [1:0] n3351;
  wire [1:0] n3352;
  wire n3353;
  wire n3354;
  wire n3355;
  wire [1:0] n3356;
  wire [1:0] n3357;
  wire n3360;
  wire n3368;
  wire n3370;
  wire n3372;
  wire n3373;
  wire n3374;
  wire n3375;
  wire n3376;
  wire n3377;
  wire n3378;
  wire n3379;
  wire n3380;
  wire n3381;
  wire n3382;
  wire n3383;
  wire n3384;
  wire n3385;
  wire n3386;
  wire n3387;
  wire n3388;
  wire n3389;
  wire n3390;
  wire n3391;
  wire n3392;
  wire n3393;
  wire n3394;
  wire n3395;
  wire n3396;
  wire n3397;
  wire n3398;
  wire n3399;
  wire n3400;
  wire n3401;
  wire n3402;
  wire n3403;
  wire n3404;
  wire n3405;
  wire n3406;
  wire n3407;
  wire n3408;
  wire n3409;
  wire n3410;
  wire n3411;
  wire [3:0] n3413;
  wire [3:0] n3414;
  wire [9:0] n3415;
  reg [3:0] n3416;
  wire [80:0] n3417;
  reg [80:0] n3418;
  wire [31:0] n3419;
  reg [31:0] n3420;
  wire n3423;
  reg n3424;
  wire [31:0] n3425;
  reg [31:0] n3426;
  wire n3427;
  reg n3428;
  wire n3429;
  reg n3430;
  wire [2:0] n3431;
  reg [2:0] n3432;
  wire n3433;
  reg n3434;
  reg n3435;
  reg n3436;
  reg [31:0] n3437;
  wire n3438;
  reg n3439;
  wire n3440;
  reg n3441;
  wire n3442;
  reg n3443;
  wire n3444;
  reg n3445;
  reg n3446;
  reg n3447;
  wire n3448;
  reg n3449;
  wire n3450;
  reg n3451;
  reg [11:0] n3452;
  wire [1:0] n3453;
  reg [1:0] n3454;
  wire [64:0] n3457;
  wire [14:0] n3460;
  wire [2:0] n3466;
  wire [84:0] n3467;
  reg n3468;
  reg n3469;
  reg n3470;
  reg n3471;
  wire [1:0] n3472;
  reg [1:0] n3473;
  wire n3475;
  wire [3:0] n3477;
  wire n3479;
  wire n3480;
  wire [30:0] n3482;
  wire [31:0] n3484;
  wire [30:0] n3485;
  wire [31:0] n3487;
  wire [30:0] n3488;
  wire [31:0] n3490;
  wire n3491;
  wire n3499;
  wire n3501;
  wire n3503;
  wire n3504;
  wire n3505;
  wire n3506;
  wire n3507;
  wire n3508;
  wire n3509;
  wire n3510;
  wire n3511;
  wire n3512;
  wire n3513;
  wire n3514;
  wire n3515;
  wire n3516;
  wire n3517;
  wire n3518;
  wire n3519;
  wire n3520;
  wire [4:0] n3521;
  wire [4:0] n3522;
  wire [4:0] n3523;
  wire n3524;
  wire [2:0] n3525;
  wire n3526;
  wire n3527;
  wire n3528;
  wire n3529;
  wire [31:0] n3530;
  wire n3531;
  wire n3539;
  wire n3541;
  wire n3543;
  wire n3544;
  wire n3545;
  wire n3546;
  wire n3547;
  wire n3548;
  wire n3549;
  wire n3557;
  wire n3559;
  wire n3561;
  wire n3562;
  wire n3563;
  wire n3564;
  wire n3565;
  wire n3566;
  wire n3567;
  wire n3575;
  wire n3577;
  wire n3579;
  wire n3580;
  wire n3581;
  wire n3582;
  wire n3583;
  wire n3584;
  wire n3585;
  wire n3586;
  wire n3587;
  wire [3:0] n3589;
  wire n3591;
  wire n3592;
  wire [3:0] n3595;
  wire n3597;
  wire n3598;
  wire n3600;
  wire n3601;
  wire n3602;
  wire n3603;
  wire n3604;
  wire n3605;
  wire [11:0] n3606;
  wire [2:0] n3607;
  wire [11:0] n3608;
  wire [6:0] n3609;
  wire [15:0] n3610;
  wire n3611;
  wire n3612;
  wire n3613;
  wire n3614;
  wire [1:0] n3615;
  wire [3:0] n3617;
  wire n3619;
  wire n3620;
  wire [3:0] n3624;
  wire n3626;
  wire n3627;
  wire [3:0] n3630;
  wire n3632;
  wire n3633;
  wire n3634;
  wire n3635;
  wire [3:0] n3638;
  wire n3640;
  wire n3641;
  wire n3642;
  wire n3643;
  wire n3644;
  wire [3:0] n3647;
  wire n3649;
  wire n3650;
  wire [3:0] n3653;
  wire n3655;
  wire n3656;
  wire n3659;
  wire [4:0] n3660;
  wire n3662;
  wire n3663;
  wire n3664;
  wire n3667;
  wire n3668;
  wire n3669;
  wire n3670;
  wire n3673;
  wire n3674;
  wire n3675;
  wire n3676;
  wire n3679;
  wire n3680;
  wire [3:0] n3681;
  wire n3683;
  wire n3684;
  wire n3685;
  wire [11:0] n3688;
  wire n3692;
  wire n3694;
  wire n3695;
  wire n3697;
  wire n3698;
  wire n3701;
  wire n3703;
  wire n3704;
  wire n3706;
  wire n3707;
  wire n3709;
  wire n3710;
  wire n3712;
  wire n3713;
  wire n3715;
  wire n3716;
  wire n3718;
  wire n3719;
  wire n3721;
  wire n3722;
  wire n3724;
  wire n3725;
  wire n3727;
  wire n3728;
  wire n3730;
  wire n3731;
  wire n3733;
  wire n3734;
  wire n3736;
  wire n3737;
  wire n3739;
  wire n3740;
  wire n3742;
  wire n3743;
  wire n3745;
  wire n3746;
  wire n3748;
  wire n3749;
  wire n3751;
  wire n3752;
  wire n3756;
  wire n3758;
  wire n3759;
  wire n3761;
  wire n3762;
  wire n3766;
  wire n3768;
  wire n3769;
  wire n3771;
  wire n3772;
  wire n3774;
  wire n3775;
  wire n3777;
  wire n3778;
  wire n3780;
  wire n3781;
  wire n3783;
  wire n3784;
  wire n3786;
  wire n3787;
  wire n3789;
  wire n3790;
  wire n3792;
  wire n3793;
  wire n3795;
  wire n3796;
  wire n3798;
  wire n3799;
  wire n3801;
  wire n3802;
  wire n3804;
  wire n3805;
  wire n3807;
  wire n3808;
  wire n3810;
  wire n3811;
  wire n3813;
  wire n3814;
  wire n3816;
  wire n3817;
  wire n3819;
  wire n3820;
  wire n3822;
  wire n3823;
  wire n3827;
  wire n3829;
  wire n3830;
  wire n3832;
  wire n3833;
  wire n3835;
  wire n3836;
  wire n3838;
  wire n3839;
  wire n3841;
  wire n3842;
  wire n3844;
  wire n3845;
  wire n3847;
  wire n3848;
  wire n3850;
  wire n3851;
  wire n3853;
  wire n3854;
  wire n3856;
  wire n3857;
  wire n3859;
  wire n3860;
  wire n3862;
  wire n3863;
  wire n3865;
  wire n3866;
  wire n3868;
  wire n3869;
  wire n3871;
  wire n3872;
  wire n3874;
  wire n3875;
  wire n3877;
  wire n3878;
  wire n3880;
  wire n3881;
  wire n3883;
  wire n3884;
  wire n3886;
  wire n3887;
  wire n3889;
  wire n3890;
  wire n3892;
  wire n3893;
  wire n3895;
  wire n3896;
  wire n3898;
  wire n3899;
  wire n3901;
  wire n3902;
  wire n3904;
  wire n3905;
  wire n3907;
  wire n3908;
  wire n3910;
  wire n3911;
  wire n3913;
  wire n3914;
  wire n3916;
  wire n3917;
  wire n3919;
  wire n3920;
  wire n3922;
  wire n3923;
  wire n3925;
  wire n3926;
  wire n3928;
  wire n3929;
  wire n3931;
  wire n3932;
  wire n3934;
  wire n3935;
  wire n3937;
  wire n3938;
  wire n3940;
  wire n3941;
  wire n3943;
  wire n3944;
  wire n3946;
  wire n3947;
  wire n3949;
  wire n3950;
  wire n3952;
  wire n3953;
  wire n3955;
  wire n3956;
  wire n3958;
  wire n3959;
  wire n3961;
  wire n3962;
  wire n3964;
  wire n3965;
  wire n3967;
  wire n3968;
  wire n3970;
  wire n3971;
  wire n3973;
  wire n3974;
  wire n3976;
  wire n3977;
  wire n3979;
  wire n3980;
  wire n3982;
  wire n3983;
  wire n3985;
  wire n3986;
  wire n3988;
  wire n3989;
  wire n3991;
  wire n3992;
  wire n3994;
  wire n3995;
  wire n3997;
  wire n3998;
  wire n4000;
  wire n4001;
  wire n4003;
  wire n4004;
  wire n4006;
  wire n4007;
  wire n4009;
  wire n4010;
  wire n4012;
  wire n4013;
  wire n4015;
  wire n4016;
  wire n4018;
  wire n4019;
  wire n4021;
  wire n4022;
  wire n4024;
  wire n4025;
  wire n4027;
  wire n4028;
  wire n4030;
  wire n4031;
  wire n4033;
  wire n4034;
  wire n4036;
  wire n4037;
  wire n4039;
  wire n4040;
  wire n4042;
  wire n4043;
  wire n4045;
  wire n4046;
  wire n4048;
  wire n4049;
  wire n4051;
  wire n4052;
  wire n4054;
  wire n4055;
  wire n4057;
  wire n4058;
  wire n4060;
  wire n4061;
  wire n4063;
  wire n4064;
  wire n4066;
  wire n4067;
  wire n4069;
  wire n4070;
  wire n4072;
  wire n4073;
  wire n4075;
  wire n4076;
  wire n4078;
  wire n4079;
  wire n4081;
  wire n4082;
  wire n4084;
  wire n4085;
  wire n4087;
  wire n4088;
  wire n4090;
  wire n4091;
  wire n4093;
  wire n4094;
  wire n4096;
  wire n4097;
  wire n4099;
  wire n4100;
  wire n4102;
  wire n4103;
  wire n4105;
  wire n4106;
  wire n4108;
  wire n4109;
  wire n4111;
  wire n4112;
  wire n4114;
  wire n4115;
  wire n4117;
  wire n4118;
  wire n4120;
  wire n4121;
  wire n4123;
  wire n4124;
  wire n4126;
  wire n4127;
  wire n4129;
  wire n4130;
  wire n4132;
  wire n4133;
  wire n4135;
  wire n4136;
  wire n4138;
  wire n4139;
  wire n4141;
  wire n4142;
  wire n4144;
  wire n4145;
  wire n4147;
  wire n4148;
  wire n4150;
  wire n4151;
  wire n4153;
  wire n4154;
  wire n4156;
  wire n4157;
  wire n4159;
  wire n4160;
  wire n4162;
  wire n4163;
  wire n4165;
  wire n4166;
  wire n4168;
  wire n4169;
  wire n4171;
  wire n4172;
  wire n4174;
  wire n4175;
  wire n4177;
  wire n4178;
  wire n4180;
  wire n4181;
  wire n4183;
  wire n4184;
  wire n4186;
  wire n4187;
  wire n4189;
  wire n4190;
  wire n4192;
  wire n4193;
  wire n4195;
  wire n4196;
  wire n4198;
  wire n4199;
  wire n4201;
  wire n4202;
  wire n4204;
  wire n4205;
  wire n4207;
  wire n4208;
  wire n4210;
  wire n4211;
  wire n4213;
  wire n4214;
  wire n4216;
  wire n4217;
  wire n4219;
  wire n4220;
  wire n4222;
  wire n4223;
  wire n4225;
  wire n4226;
  wire n4228;
  wire n4229;
  wire n4231;
  wire n4232;
  wire n4234;
  wire n4235;
  wire n4237;
  wire n4238;
  wire n4240;
  wire n4241;
  wire n4243;
  wire n4244;
  wire n4246;
  wire n4247;
  wire n4249;
  wire n4250;
  wire n4252;
  wire n4253;
  wire n4255;
  wire n4256;
  wire n4258;
  wire n4259;
  wire n4261;
  wire n4262;
  wire n4264;
  wire n4265;
  wire n4267;
  wire n4268;
  wire n4270;
  wire n4271;
  wire n4273;
  wire n4274;
  wire n4276;
  wire n4277;
  wire n4279;
  wire n4280;
  wire n4282;
  wire n4283;
  wire n4285;
  wire n4286;
  wire n4288;
  wire n4289;
  wire n4291;
  wire n4292;
  wire n4294;
  wire n4295;
  wire n4297;
  wire n4298;
  wire n4300;
  wire n4301;
  wire n4303;
  wire n4304;
  wire n4306;
  wire n4307;
  wire n4309;
  wire n4310;
  wire n4312;
  wire n4313;
  wire n4315;
  wire n4316;
  wire n4318;
  wire n4319;
  wire n4321;
  wire n4322;
  wire n4324;
  wire n4325;
  wire n4327;
  wire n4328;
  wire n4330;
  wire n4331;
  wire n4333;
  wire n4334;
  wire n4336;
  wire n4337;
  wire n4339;
  wire n4340;
  wire n4342;
  wire n4343;
  wire n4345;
  wire n4346;
  wire n4350;
  wire n4352;
  wire n4353;
  wire n4355;
  wire n4356;
  wire n4358;
  wire n4359;
  wire n4361;
  wire n4362;
  wire n4364;
  wire n4365;
  wire n4367;
  wire n4368;
  wire n4370;
  wire n4371;
  wire n4373;
  wire n4374;
  wire n4376;
  wire n4377;
  wire n4381;
  wire n4383;
  wire n4384;
  wire n4386;
  wire n4387;
  wire n4389;
  wire n4390;
  wire n4394;
  wire n4396;
  wire n4397;
  wire n4399;
  wire n4400;
  wire n4404;
  wire n4406;
  wire n4407;
  wire n4409;
  wire n4410;
  wire n4412;
  wire n4413;
  wire n4415;
  wire n4416;
  wire [8:0] n4418;
  reg n4419;
  wire [1:0] n4420;
  wire n4422;
  wire [2:0] n4423;
  wire n4425;
  wire [2:0] n4426;
  wire n4428;
  wire n4429;
  wire [4:0] n4430;
  wire n4432;
  wire n4433;
  wire n4434;
  wire n4437;
  wire [1:0] n4441;
  wire n4443;
  wire n4444;
  wire n4445;
  wire n4446;
  wire n4449;
  wire [6:0] n4452;
  wire n4454;
  wire n4456;
  wire n4457;
  wire n4459;
  wire n4460;
  wire [2:0] n4461;
  wire n4463;
  wire n4466;
  wire n4468;
  wire [1:0] n4469;
  wire n4471;
  wire n4473;
  wire n4476;
  wire n4478;
  wire [2:0] n4479;
  wire n4481;
  wire n4483;
  wire n4484;
  wire n4486;
  wire n4487;
  wire n4489;
  wire n4490;
  wire n4492;
  wire n4493;
  reg n4496;
  wire n4498;
  wire [2:0] n4499;
  wire n4501;
  wire n4503;
  wire n4504;
  wire n4506;
  wire n4507;
  reg n4510;
  wire n4512;
  wire n4552;
  wire n4554;
  wire n4556;
  wire n4557;
  wire n4559;
  wire n4560;
  wire n4562;
  wire n4563;
  wire n4565;
  wire n4566;
  wire n4568;
  wire n4569;
  wire n4571;
  wire n4572;
  wire [1:0] n4573;
  wire n4575;
  wire n4578;
  wire n4580;
  wire [2:0] n4581;
  wire n4583;
  wire [4:0] n4584;
  wire n4586;
  wire [4:0] n4587;
  wire n4589;
  wire n4590;
  wire [11:0] n4591;
  wire n4593;
  wire n4595;
  wire n4596;
  wire n4597;
  wire n4598;
  wire n4599;
  wire n4601;
  wire n4602;
  wire n4603;
  wire n4605;
  wire n4606;
  wire n4607;
  wire n4608;
  wire n4609;
  wire n4611;
  wire [4:0] n4612;
  reg n4616;
  wire n4618;
  wire [2:0] n4619;
  wire n4621;
  wire n4624;
  wire n4627;
  wire n4629;
  wire n4630;
  wire n4632;
  wire [8:0] n4633;
  reg n4638;
  wire n4642;
  wire [3:0] n4644;
  wire n4646;
  wire [9:0] n4648;
  wire [9:0] n4650;
  wire [3:0] n4656;
  wire n4658;
  wire [3:0] n4659;
  wire n4661;
  wire n4662;
  wire n4663;
  wire n4664;
  wire n4665;
  wire n4666;
  wire n4669;
  wire [16:0] n4675;
  wire [19:0] n4676;
  wire n4677;
  wire n4678;
  wire n4679;
  wire n4680;
  wire n4681;
  wire n4682;
  wire n4683;
  wire n4684;
  wire n4685;
  wire n4686;
  wire n4687;
  wire n4688;
  wire n4689;
  wire n4690;
  wire n4691;
  wire n4692;
  wire n4693;
  wire n4694;
  wire n4695;
  wire n4696;
  wire n4697;
  wire n4698;
  wire n4699;
  wire n4700;
  wire n4701;
  wire n4702;
  wire n4703;
  wire n4704;
  wire n4705;
  wire n4706;
  wire n4707;
  wire n4708;
  wire n4709;
  wire n4710;
  wire n4711;
  wire n4712;
  wire n4713;
  wire n4714;
  wire n4715;
  wire n4716;
  wire n4717;
  wire n4718;
  wire n4719;
  wire n4720;
  wire n4721;
  wire n4722;
  wire n4723;
  wire n4724;
  wire n4725;
  wire n4726;
  wire n4727;
  wire n4728;
  wire n4729;
  wire n4730;
  wire n4731;
  wire n4732;
  wire n4733;
  wire n4734;
  wire n4735;
  wire n4736;
  wire n4737;
  wire n4738;
  wire n4739;
  wire n4740;
  wire n4741;
  wire n4742;
  wire n4743;
  wire n4744;
  wire n4745;
  wire n4746;
  wire n4747;
  wire n4748;
  wire n4749;
  wire n4750;
  wire n4751;
  wire n4752;
  wire n4753;
  wire n4754;
  wire n4755;
  wire n4756;
  wire n4757;
  wire n4758;
  wire n4759;
  wire n4760;
  wire n4761;
  wire n4762;
  wire n4763;
  wire n4764;
  wire n4765;
  wire n4766;
  wire n4767;
  wire n4768;
  wire n4769;
  wire n4770;
  wire n4771;
  wire n4772;
  wire n4773;
  wire n4774;
  wire n4775;
  wire n4776;
  wire n4777;
  wire n4778;
  wire n4779;
  wire n4780;
  wire n4781;
  wire n4782;
  wire n4783;
  wire n4784;
  wire n4785;
  wire n4786;
  wire n4787;
  wire n4788;
  wire n4789;
  wire n4790;
  wire n4791;
  wire n4792;
  wire n4793;
  wire n4794;
  wire n4795;
  wire n4796;
  wire n4797;
  wire n4798;
  wire n4799;
  wire n4800;
  wire n4801;
  wire n4802;
  wire n4803;
  wire n4804;
  wire n4805;
  wire n4806;
  wire n4807;
  wire n4808;
  wire n4809;
  wire n4810;
  wire n4811;
  wire n4812;
  wire n4813;
  wire n4814;
  wire n4815;
  wire n4816;
  wire n4817;
  wire n4818;
  wire n4819;
  wire n4820;
  wire n4821;
  wire n4822;
  wire n4823;
  wire n4824;
  wire n4825;
  wire n4826;
  wire n4827;
  wire n4828;
  wire n4829;
  wire n4830;
  wire n4831;
  wire n4832;
  wire n4833;
  wire n4834;
  wire n4835;
  wire n4836;
  wire n4837;
  wire n4838;
  wire n4839;
  wire n4840;
  wire n4841;
  wire n4842;
  wire n4843;
  wire n4844;
  wire n4845;
  wire n4846;
  wire n4847;
  wire n4848;
  wire n4849;
  wire n4850;
  wire n4851;
  wire n4852;
  wire n4853;
  wire n4854;
  wire n4855;
  wire n4856;
  wire n4857;
  wire n4858;
  wire n4859;
  wire n4860;
  wire n4861;
  wire n4862;
  wire n4863;
  wire n4864;
  wire n4865;
  wire n4866;
  wire n4867;
  wire n4868;
  wire n4869;
  wire n4870;
  wire n4871;
  wire n4872;
  wire n4873;
  wire n4874;
  wire n4875;
  wire n4876;
  wire n4877;
  wire n4878;
  wire n4879;
  wire n4880;
  wire n4881;
  wire n4882;
  wire n4883;
  wire n4884;
  wire n4885;
  wire [11:0] n4886;
  wire [39:0] n4887;
  wire [39:0] n4892;
  wire n4896;
  wire n4897;
  wire n4898;
  wire n4899;
  wire n4900;
  wire n4901;
  wire n4902;
  wire n4903;
  wire n4904;
  wire n4905;
  wire n4906;
  wire n4907;
  wire n4908;
  wire n4909;
  wire n4910;
  wire n4911;
  wire n4912;
  wire n4913;
  wire n4914;
  wire n4915;
  wire n4917;
  wire n4920;
  wire n4922;
  wire n4930;
  wire n4932;
  wire n4934;
  wire n4935;
  wire n4936;
  wire n4938;
  wire n4939;
  wire n4940;
  wire n4952;
  wire n4954;
  wire n4956;
  wire n4957;
  wire n4958;
  wire n4959;
  wire n4960;
  wire n4961;
  wire n4962;
  wire n4963;
  wire n4964;
  wire n4965;
  wire n4966;
  wire n4967;
  wire n4968;
  wire n4969;
  wire n4970;
  wire n4971;
  wire n4972;
  wire n4973;
  wire n4974;
  wire n4975;
  wire n4976;
  wire n4977;
  wire [3:0] n4979;
  wire n4981;
  wire [3:0] n4982;
  wire n4984;
  wire n4985;
  wire n4993;
  wire n4995;
  wire n4997;
  wire n4998;
  wire n4999;
  wire n5000;
  wire n5001;
  wire n5002;
  wire n5003;
  wire n5004;
  wire n5005;
  wire n5006;
  wire n5007;
  wire n5008;
  wire n5009;
  wire n5010;
  wire n5011;
  wire n5012;
  wire n5013;
  wire n5014;
  wire n5015;
  wire n5016;
  wire n5017;
  wire n5018;
  wire n5019;
  wire n5020;
  wire n5021;
  wire n5022;
  wire n5023;
  wire n5024;
  wire n5025;
  wire n5026;
  wire n5027;
  wire n5028;
  wire n5029;
  wire n5030;
  wire n5031;
  wire n5032;
  wire n5033;
  wire n5034;
  wire n5035;
  wire n5036;
  wire n5037;
  wire n5038;
  wire n5039;
  wire n5040;
  wire n5041;
  wire n5042;
  wire n5043;
  wire n5044;
  wire n5045;
  wire n5047;
  wire n5049;
  wire [6:0] n5050;
  wire n5052;
  wire [6:0] n5053;
  wire n5055;
  wire [6:0] n5056;
  wire n5057;
  wire [6:0] n5058;
  wire n5060;
  wire [6:0] n5061;
  wire n5063;
  wire [6:0] n5064;
  wire n5066;
  wire [6:0] n5067;
  wire n5069;
  wire [6:0] n5070;
  wire n5072;
  wire [6:0] n5073;
  wire n5075;
  wire [6:0] n5076;
  wire n5078;
  wire [6:0] n5079;
  wire n5081;
  wire [6:0] n5082;
  wire n5084;
  wire [6:0] n5085;
  wire n5087;
  wire [6:0] n5088;
  wire n5090;
  wire [6:0] n5091;
  wire n5093;
  wire [6:0] n5094;
  wire n5096;
  wire [6:0] n5097;
  wire n5099;
  wire [6:0] n5100;
  wire n5102;
  wire [6:0] n5103;
  wire n5105;
  wire [6:0] n5106;
  wire n5108;
  wire [6:0] n5109;
  wire n5111;
  wire [6:0] n5112;
  wire n5114;
  wire [6:0] n5115;
  wire n5117;
  wire [6:0] n5118;
  wire n5120;
  wire [6:0] n5121;
  wire n5123;
  wire [6:0] n5124;
  wire n5126;
  wire [6:0] n5127;
  wire n5129;
  wire [6:0] n5130;
  wire n5132;
  wire [6:0] n5133;
  wire n5135;
  wire [6:0] n5136;
  wire n5138;
  wire [6:0] n5139;
  wire n5141;
  wire [5:0] n5143;
  wire n5144;
  wire [6:0] n5145;
  wire [31:0] n5146;
  wire n5147;
  wire [31:0] n5148;
  wire [31:0] n5149;
  wire n5177;
  wire n5178;
  wire [4:0] n5180;
  wire [31:0] n5182;
  wire [31:0] n5183;
  wire [1:0] n5184;
  wire [31:0] n5185;
  wire n5187;
  wire [31:0] n5188;
  wire [31:0] n5189;
  wire n5191;
  wire [1:0] n5192;
  reg [31:0] n5193;
  wire n5196;
  wire n5221;
  wire [11:0] n5222;
  wire n5223;
  wire n5224;
  wire n5232;
  wire n5234;
  wire n5236;
  wire n5237;
  wire n5238;
  wire n5239;
  wire n5241;
  wire n5242;
  wire n5243;
  wire n5244;
  wire [15:0] n5245;
  wire n5247;
  wire [29:0] n5248;
  wire [30:0] n5250;
  wire n5251;
  wire [31:0] n5252;
  wire n5254;
  wire n5256;
  wire n5258;
  wire [30:0] n5259;
  wire [31:0] n5261;
  wire n5263;
  wire n5264;
  wire [4:0] n5265;
  wire [5:0] n5266;
  wire n5268;
  wire n5270;
  wire n5271;
  wire n5272;
  wire n5273;
  wire n5281;
  wire n5283;
  wire n5285;
  wire n5286;
  wire n5288;
  wire n5293;
  wire n5295;
  wire [10:0] n5296;
  wire n5297;
  reg n5298;
  wire n5299;
  reg n5300;
  wire n5301;
  reg n5302;
  wire n5303;
  reg n5304;
  wire n5305;
  reg n5306;
  wire n5307;
  reg n5308;
  wire n5309;
  reg n5310;
  wire n5311;
  reg n5312;
  wire [15:0] n5313;
  reg [15:0] n5314;
  wire [31:0] n5315;
  reg [31:0] n5316;
  wire [5:0] n5317;
  reg [5:0] n5318;
  wire [31:0] n5319;
  reg [31:0] n5320;
  wire [31:0] n5321;
  reg [31:0] n5322;
  wire [31:0] n5323;
  reg [31:0] n5324;
  wire [31:0] n5325;
  reg [31:0] n5326;
  wire n5327;
  reg n5328;
  wire n5329;
  reg n5330;
  wire n5331;
  reg n5332;
  wire n5333;
  reg n5334;
  wire n5339;
  wire n5340;
  wire n5341;
  wire n5343;
  wire n5345;
  wire n5346;
  wire [4:0] n5347;
  wire [5:0] n5348;
  wire [30:0] n5349;
  wire [31:0] n5351;
  wire n5352;
  wire n5353;
  wire n5354;
  wire [1:0] n5355;
  wire n5357;
  wire n5358;
  wire n5360;
  wire [15:0] n5361;
  wire [31:0] n5363;
  wire [31:0] n5364;
  wire [31:0] n5365;
  wire [31:0] n5367;
  wire [31:0] n5368;
  wire [31:0] n5370;
  wire [3:0] n5371;
  wire [37:0] n5372;
  wire [3:0] n5373;
  wire [3:0] n5374;
  wire [37:0] n5375;
  wire [37:0] n5376;
  wire [31:0] n5377;
  wire [31:0] n5378;
  wire n5379;
  wire n5380;
  wire n5381;
  wire n5383;
  wire n5385;
  wire n5386;
  wire n5388;
  wire [4:0] n5390;
  wire [4:0] n5391;
  wire [4:0] n5392;
  wire [3:0] n5393;
  wire [3:0] n5394;
  wire n5395;
  wire n5396;
  wire n5397;
  wire n5399;
  wire n5401;
  wire [4:0] n5402;
  wire [193:0] n5403;
  wire [3:0] n5408;
  wire [3:0] n5409;
  wire [3:0] n5410;
  wire [19:0] n5411;
  wire [19:0] n5412;
  wire [19:0] n5413;
  wire [37:0] n5414;
  wire [37:0] n5415;
  wire [31:0] n5416;
  wire [31:0] n5417;
  wire [31:0] n5418;
  wire [31:0] n5419;
  wire [31:0] n5420;
  wire [67:0] n5421;
  wire [67:0] n5422;
  wire [67:0] n5423;
  wire [1:0] n5432;
  wire [18:0] n5435;
  wire [31:0] n5442;
  wire [261:0] n5451;
  wire [261:0] n5453;
  wire n5457;
  wire n5459;
  wire [11:0] n5460;
  wire n5461;
  wire n5462;
  wire n5463;
  wire n5464;
  wire n5465;
  wire n5466;
  wire n5469;
  wire n5471;
  localparam [1:0] n5487 = 2'b01;
  wire n5489;
  wire n5490;
  wire n5491;
  wire n5492;
  wire [15:0] n5493;
  wire n5495;
  wire [31:0] n5496;
  wire n5498;
  wire n5500;
  wire [31:0] n5501;
  wire n5503;
  wire [30:0] n5504;
  wire [31:0] n5506;
  wire n5508;
  wire n5509;
  wire [4:0] n5510;
  wire n5512;
  wire [31:0] n5513;
  wire n5515;
  wire n5516;
  wire n5517;
  wire n5518;
  wire [15:0] n5519;
  wire n5521;
  wire n5523;
  wire n5525;
  wire n5527;
  wire n5529;
  wire n5531;
  wire n5533;
  wire n5535;
  wire n5599;
  wire n5603;
  wire [18:0] n5604;
  wire n5605;
  wire n5606;
  wire n5607;
  wire n5608;
  wire n5609;
  wire n5614;
  reg n5616;
  wire n5617;
  wire n5618;
  wire n5619;
  wire n5620;
  wire n5621;
  wire n5626;
  reg n5628;
  wire n5629;
  wire n5630;
  wire n5631;
  wire n5632;
  wire n5633;
  wire n5638;
  reg n5640;
  wire n5641;
  wire n5642;
  wire n5643;
  wire n5644;
  wire n5645;
  wire n5650;
  reg n5652;
  wire n5653;
  wire n5654;
  wire n5655;
  wire n5656;
  wire n5657;
  wire n5662;
  reg n5664;
  wire n5665;
  wire n5666;
  wire n5667;
  wire n5668;
  wire n5673;
  reg n5675;
  wire n5676;
  wire n5677;
  wire n5678;
  wire n5679;
  wire n5684;
  reg n5686;
  wire n5687;
  wire n5688;
  wire n5689;
  wire n5690;
  wire n5695;
  reg n5697;
  wire n5698;
  wire n5699;
  wire n5700;
  wire n5701;
  wire n5706;
  reg n5708;
  wire n5709;
  wire n5710;
  wire n5711;
  wire n5712;
  wire n5717;
  reg n5719;
  wire n5720;
  wire n5721;
  wire n5722;
  wire n5723;
  wire n5728;
  reg n5730;
  wire n5731;
  wire n5732;
  wire n5733;
  wire n5734;
  wire n5739;
  reg n5741;
  wire n5742;
  wire n5743;
  wire n5744;
  wire n5745;
  wire n5750;
  reg n5752;
  wire n5753;
  wire n5754;
  wire n5755;
  wire n5756;
  wire n5761;
  reg n5763;
  wire n5764;
  wire n5765;
  wire n5766;
  wire n5767;
  wire n5772;
  reg n5774;
  wire n5775;
  wire n5776;
  wire n5777;
  wire n5778;
  wire n5783;
  reg n5785;
  wire n5786;
  wire n5787;
  wire n5788;
  wire n5789;
  wire n5790;
  wire n5791;
  wire n5796;
  reg n5798;
  wire n5799;
  wire n5800;
  wire n5801;
  wire n5802;
  wire n5803;
  wire n5804;
  wire n5809;
  reg n5811;
  wire n5812;
  wire n5813;
  wire n5814;
  wire n5815;
  wire n5816;
  wire n5817;
  wire n5822;
  reg n5824;
  wire n5825;
  wire n5826;
  wire n5827;
  wire n5828;
  wire n5829;
  wire n5830;
  wire n5835;
  reg n5837;
  wire n5838;
  wire n5839;
  wire n5840;
  wire n5841;
  wire n5842;
  wire n5843;
  wire n5848;
  reg n5850;
  wire n5851;
  wire n5852;
  wire n5853;
  wire n5854;
  wire n5855;
  wire n5856;
  wire n5861;
  reg n5863;
  wire n5864;
  wire n5865;
  wire n5866;
  wire n5867;
  wire n5868;
  wire n5869;
  wire n5874;
  reg n5876;
  wire n5877;
  wire n5878;
  wire n5879;
  wire n5880;
  wire n5881;
  wire n5882;
  wire n5887;
  reg n5889;
  wire n5890;
  wire n5891;
  wire n5892;
  wire n5893;
  wire n5894;
  wire n5895;
  wire n5900;
  reg n5902;
  wire n5903;
  wire n5904;
  wire n5905;
  wire n5906;
  wire n5907;
  wire n5908;
  wire n5913;
  reg n5915;
  wire n5916;
  wire n5917;
  wire n5918;
  wire n5919;
  wire n5920;
  wire n5921;
  wire n5926;
  reg n5928;
  wire n5929;
  wire n5930;
  wire n5931;
  wire n5932;
  wire n5933;
  wire n5934;
  wire n5939;
  reg n5941;
  wire n5942;
  wire n5943;
  wire n5944;
  wire n5945;
  wire n5946;
  wire n5947;
  wire n5952;
  reg n5954;
  wire n5955;
  wire n5956;
  wire n5957;
  wire n5958;
  wire n5959;
  wire n5960;
  wire n5965;
  reg n5967;
  wire n5968;
  wire n5969;
  wire n5970;
  wire n5971;
  wire n5972;
  wire n5973;
  wire n5974;
  wire n5979;
  reg n5981;
  wire n5982;
  wire n5983;
  wire n5984;
  wire n5985;
  wire n5986;
  wire n5987;
  wire n5988;
  wire n5993;
  reg n5995;
  wire [31:0] n5996;
  wire [31:0] n5998;
  reg [116:0] n6004;
  wire [116:0] n6005;
  reg [263:0] n6006;
  wire [263:0] n6007;
  reg n6008;
  reg [39:0] n6009;
  reg [11:0] n6010;
  wire [101:0] n6011;
  reg [261:0] n6012;
  reg [31:0] n6013;
  wire [4:0] n6015;
  reg [9:0] n6016;
  wire [2:0] n6017;
  wire [10:0] n6018;
  wire [263:0] n6019;
  assign \ctrl_o_ctrl_o[if_reset]  = n2718; //(module output)
  assign \ctrl_o_ctrl_o[if_ready]  = n2719; //(module output)
  assign \ctrl_o_ctrl_o[pc_cur]  = n2720; //(module output)
  assign \ctrl_o_ctrl_o[pc_nxt]  = n2721; //(module output)
  assign \ctrl_o_ctrl_o[pc_ret]  = n2722; //(module output)
  assign \ctrl_o_ctrl_o[rf_wb_en]  = n2723; //(module output)
  assign \ctrl_o_ctrl_o[rf_rs1]  = n2724; //(module output)
  assign \ctrl_o_ctrl_o[rf_rs2]  = n2725; //(module output)
  assign \ctrl_o_ctrl_o[rf_rd]  = n2726; //(module output)
  assign \ctrl_o_ctrl_o[rf_zero]  = n2727; //(module output)
  assign \ctrl_o_ctrl_o[alu_op]  = n2728; //(module output)
  assign \ctrl_o_ctrl_o[alu_sub]  = n2729; //(module output)
  assign \ctrl_o_ctrl_o[alu_opa_mux]  = n2730; //(module output)
  assign \ctrl_o_ctrl_o[alu_opb_mux]  = n2731; //(module output)
  assign \ctrl_o_ctrl_o[alu_unsigned]  = n2732; //(module output)
  assign \ctrl_o_ctrl_o[alu_imm]  = n2733; //(module output)
  assign \ctrl_o_ctrl_o[alu_cp_alu]  = n2734; //(module output)
  assign \ctrl_o_ctrl_o[alu_cp_cfu]  = n2735; //(module output)
  assign \ctrl_o_ctrl_o[alu_cp_fpu]  = n2736; //(module output)
  assign \ctrl_o_ctrl_o[lsu_req]  = n2737; //(module output)
  assign \ctrl_o_ctrl_o[lsu_rd]  = n2738; //(module output)
  assign \ctrl_o_ctrl_o[lsu_wr]  = n2739; //(module output)
  assign \ctrl_o_ctrl_o[lsu_mo_en]  = n2740; //(module output)
  assign \ctrl_o_ctrl_o[lsu_mi_en]  = n2741; //(module output)
  assign \ctrl_o_ctrl_o[lsu_priv]  = n2742; //(module output)
  assign \ctrl_o_ctrl_o[csr_we]  = n2743; //(module output)
  assign \ctrl_o_ctrl_o[csr_re]  = n2744; //(module output)
  assign \ctrl_o_ctrl_o[csr_addr]  = n2745; //(module output)
  assign \ctrl_o_ctrl_o[csr_wdata]  = n2746; //(module output)
  assign \ctrl_o_ctrl_o[cnt_event]  = n2747; //(module output)
  assign \ctrl_o_ctrl_o[ir_funct3]  = n2748; //(module output)
  assign \ctrl_o_ctrl_o[ir_funct12]  = n2749; //(module output)
  assign \ctrl_o_ctrl_o[ir_opcode]  = n2750; //(module output)
  assign \ctrl_o_ctrl_o[ir_rvc]  = n2751; //(module output)
  assign \ctrl_o_ctrl_o[cpu_priv]  = n2752; //(module output)
  assign \ctrl_o_ctrl_o[cpu_trap]  = n2753; //(module output)
  assign \ctrl_o_ctrl_o[cpu_sync_exc]  = n2754; //(module output)
  assign \ctrl_o_ctrl_o[cpu_debug]  = n2755; //(module output)
  assign \ctrl_o_ctrl_o[cpu_fence]  = n2756; //(module output)
  assign csr_rdata_o = csr_rdata; //(module output)
  assign n2718 = n6019[0]; // extract
  assign n2719 = n6019[1]; // extract
  assign n2720 = n6019[33:2]; // extract
  assign n2721 = n6019[65:34]; // extract
  assign n2722 = n6019[97:66]; // extract
  assign n2723 = n6019[98]; // extract
  assign n2724 = n6019[103:99]; // extract
  assign n2725 = n6019[108:104]; // extract
  assign n2726 = n6019[113:109]; // extract
  assign n2727 = n6019[114]; // extract
  assign n2728 = n6019[117:115]; // extract
  assign n2729 = n6019[118]; // extract
  assign n2730 = n6019[119]; // extract
  assign n2731 = n6019[120]; // extract
  assign n2732 = n6019[121]; // extract
  assign n2733 = n6019[153:122]; // extract
  assign n2734 = n6019[154]; // extract
  assign n2735 = n6019[155]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:241:5  */
  assign n2736 = n6019[156]; // extract
  assign n2737 = n6019[157]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:228:5  */
  assign n2738 = n6019[158]; // extract
  assign n2739 = n6019[159]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:103:3  */
  assign n2740 = n6019[160]; // extract
  assign n2741 = n6019[161]; // extract
  assign n2742 = n6019[162]; // extract
  assign n2743 = n6019[163]; // extract
  assign n2744 = n6019[164]; // extract
  assign n2745 = n6019[176:165]; // extract
  assign n2746 = n6019[208:177]; // extract
  assign n2747 = n6019[219:209]; // extract
  assign n2748 = n6019[222:220]; // extract
  assign n2749 = n6019[234:223]; // extract
  assign n2750 = n6019[241:235]; // extract
  assign n2751 = n6019[257:242]; // extract
  assign n2752 = n6019[258]; // extract
  assign n2753 = n6019[259]; // extract
  assign n2754 = n6019[260]; // extract
  assign n2755 = n6019[261]; // extract
  assign n2756 = n6019[263:262]; // extract
  assign n2757 = {\frontend_i_frontend_i[fault] , \frontend_i_frontend_i[compr] , \frontend_i_frontend_i[i16] , \frontend_i_frontend_i[i32] , \frontend_i_frontend_i[valid] };
  /* ../../rtl/core/neorv32_cpu_control.vhd:105:10  */
  assign exec = n6004; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:105:16  */
  assign exec_nxt = n6005; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:106:10  */
  assign ctrl = n6006; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:106:16  */
  assign ctrl_nxt = n6007; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:126:10  */
  assign trap = n6011; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:154:10  */
  assign csr = n6012; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:155:10  */
  assign csr_wdata = n5193; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:155:21  */
  assign csr_rdata = n6013; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:161:10  */
  assign debug_ctrl = n6015; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:164:10  */
  assign branch_taken = n2772; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:165:10  */
  assign monitor_cnt = n6016; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:166:10  */
  assign csr_valid = n6017; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:167:10  */
  assign illegal_cmd = n4638; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:168:10  */
  assign cnt_event = n6018; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:169:10  */
  assign ebreak_trig = n4915; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:170:10  */
  assign trap_env = n5145; // (signal)
  /* ../../rtl/core/neorv32_cpu_control.vhd:182:16  */
  assign n2760 = exec[6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:182:39  */
  assign n2761 = ~n2760;
  /* ../../rtl/core/neorv32_cpu_control.vhd:183:18  */
  assign n2762 = exec[18]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:183:39  */
  assign n2763 = ~n2762;
  /* ../../rtl/core/neorv32_cpu_control.vhd:184:34  */
  assign n2764 = alu_cmp_i[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:184:49  */
  assign n2765 = exec[16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:184:38  */
  assign n2766 = n2764 ^ n2765;
  /* ../../rtl/core/neorv32_cpu_control.vhd:186:34  */
  assign n2767 = alu_cmp_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:186:49  */
  assign n2768 = exec[16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:186:38  */
  assign n2769 = n2767 ^ n2768;
  /* ../../rtl/core/neorv32_cpu_control.vhd:183:7  */
  assign n2770 = n2763 ? n2766 : n2769;
  /* ../../rtl/core/neorv32_cpu_control.vhd:182:5  */
  assign n2772 = n2761 ? n2770 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:198:16  */
  assign n2775 = ~rstn_i;
  assign n2785 = {32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 1'b0, 16'b0000000000000000, 32'b00000000000000000000000000000000, 4'b0000};
  /* ../../rtl/core/neorv32_cpu_control.vhd:221:24  */
  assign n2794 = exec[10:6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:221:73  */
  assign n2796 = {n2794, 2'b11};
  /* ../../rtl/core/neorv32_cpu_control.vhd:222:24  */
  assign n2797 = exec[35:29]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:223:24  */
  assign n2798 = exec[18:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:234:31  */
  assign n2805 = ctrl[176:165]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:235:31  */
  assign n2809 = ctrl[158]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:236:31  */
  assign n2812 = ctrl[159]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:240:70  */
  assign n2815 = exec[35]; // extract
  assign n2821 = {n2815, n2815, n2815, n2815};
  assign n2822 = {n2815, n2815, n2815, n2815};
  assign n2823 = {n2815, n2815, n2815, n2815};
  assign n2824 = {n2815, n2815, n2815, n2815};
  assign n2825 = {n2815, n2815, n2815, n2815};
  assign n2826 = {n2821, n2822, n2823, n2824};
  assign n2827 = {n2825, n2815};
  assign n2828 = {n2826, n2827};
  /* ../../rtl/core/neorv32_cpu_control.vhd:240:89  */
  assign n2830 = exec[34:29]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:240:80  */
  assign n2831 = {n2828, n2830};
  /* ../../rtl/core/neorv32_cpu_control.vhd:240:113  */
  assign n2832 = exec[15:11]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:240:104  */
  assign n2833 = {n2831, n2832};
  /* ../../rtl/core/neorv32_cpu_control.vhd:240:7  */
  assign n2835 = n2796 == 7'b0100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:241:70  */
  assign n2837 = exec[35]; // extract
  assign n2843 = {n2837, n2837, n2837, n2837};
  assign n2844 = {n2837, n2837, n2837, n2837};
  assign n2845 = {n2837, n2837, n2837, n2837};
  assign n2846 = {n2837, n2837, n2837, n2837};
  assign n2847 = {n2837, n2837, n2837, n2837};
  assign n2848 = {n2843, n2844, n2845, n2846};
  assign n2849 = {n2848, n2847};
  /* ../../rtl/core/neorv32_cpu_control.vhd:241:89  */
  assign n2851 = exec[11]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:241:80  */
  assign n2852 = {n2849, n2851};
  /* ../../rtl/core/neorv32_cpu_control.vhd:241:102  */
  assign n2853 = exec[34:29]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:241:93  */
  assign n2854 = {n2852, n2853};
  /* ../../rtl/core/neorv32_cpu_control.vhd:241:126  */
  assign n2855 = exec[15:12]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:241:117  */
  assign n2856 = {n2854, n2855};
  /* ../../rtl/core/neorv32_cpu_control.vhd:241:140  */
  assign n2858 = {n2856, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:241:7  */
  assign n2860 = n2796 == 7'b1100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:243:58  */
  assign n2861 = exec[35:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:243:73  */
  assign n2863 = {n2861, 12'b000000000000};
  /* ../../rtl/core/neorv32_cpu_control.vhd:242:7  */
  assign n2865 = n2796 == 7'b0110111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:242:25  */
  assign n2867 = n2796 == 7'b0010111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:242:25  */
  assign n2868 = n2865 | n2867;
  /* ../../rtl/core/neorv32_cpu_control.vhd:244:70  */
  assign n2870 = exec[35]; // extract
  assign n2876 = {n2870, n2870, n2870, n2870};
  assign n2877 = {n2870, n2870, n2870, n2870};
  assign n2878 = {n2870, n2870, n2870, n2870};
  assign n2879 = {n2876, n2877, n2878};
  /* ../../rtl/core/neorv32_cpu_control.vhd:244:89  */
  assign n2881 = exec[23:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:244:80  */
  assign n2882 = {n2879, n2881};
  /* ../../rtl/core/neorv32_cpu_control.vhd:244:113  */
  assign n2883 = exec[24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:244:104  */
  assign n2884 = {n2882, n2883};
  /* ../../rtl/core/neorv32_cpu_control.vhd:244:127  */
  assign n2885 = exec[34:25]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:244:118  */
  assign n2886 = {n2884, n2885};
  /* ../../rtl/core/neorv32_cpu_control.vhd:244:142  */
  assign n2888 = {n2886, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:244:7  */
  assign n2890 = n2796 == 7'b1101111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:245:7  */
  assign n2893 = n2796 == 7'b0101111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:246:70  */
  assign n2895 = exec[35]; // extract
  assign n2901 = {n2895, n2895, n2895, n2895};
  assign n2902 = {n2895, n2895, n2895, n2895};
  assign n2903 = {n2895, n2895, n2895, n2895};
  assign n2904 = {n2895, n2895, n2895, n2895};
  assign n2905 = {n2895, n2895, n2895, n2895};
  assign n2906 = {n2901, n2902, n2903, n2904};
  assign n2907 = {n2905, n2895};
  assign n2908 = {n2906, n2907};
  /* ../../rtl/core/neorv32_cpu_control.vhd:246:89  */
  assign n2910 = exec[34:25]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:246:80  */
  assign n2911 = {n2908, n2910};
  /* ../../rtl/core/neorv32_cpu_control.vhd:246:113  */
  assign n2912 = exec[24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:246:104  */
  assign n2913 = {n2911, n2912};
  assign n2914 = {n2893, n2890, n2868, n2860, n2835};
  /* ../../rtl/core/neorv32_cpu_control.vhd:239:5  */
  always @*
    case (n2914)
      5'b10000: n2915 = 32'b00000000000000000000000000000000;
      5'b01000: n2915 = n2888;
      5'b00100: n2915 = n2863;
      5'b00010: n2915 = n2858;
      5'b00001: n2915 = n2833;
      default: n2915 = n2913;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:250:17  */
  assign n2918 = n2796[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:251:40  */
  assign n2919 = exec[16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:253:40  */
  assign n2920 = exec[17]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:250:5  */
  assign n2921 = n2918 ? n2919 : n2920;
  /* ../../rtl/core/neorv32_cpu_control.vhd:257:18  */
  assign n2924 = n2796 == 7'b0010111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:257:49  */
  assign n2926 = n2796 == 7'b1101111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:257:36  */
  assign n2927 = n2924 | n2926;
  /* ../../rtl/core/neorv32_cpu_control.vhd:257:78  */
  assign n2929 = n2796 == 7'b1100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:257:65  */
  assign n2930 = n2927 | n2929;
  assign n2932 = n2806[119]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:257:5  */
  assign n2933 = n2930 ? 1'b1 : n2932;
  assign n2934 = n2806[120]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:260:18  */
  assign n2937 = n2796 != 7'b0110011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:260:5  */
  assign n2939 = n2937 ? 1'b1 : n2934;
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:15  */
  assign n2940 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:267:7  */
  assign n2945 = n2940 == 4'b0000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:278:40  */
  assign n2948 = n2757[49]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:278:24  */
  assign n2950 = n2948 & 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:278:9  */
  assign n2953 = n2950 ? 32'b00000000000000000000000000000010 : 32'b00000000000000000000000000000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:284:18  */
  assign n2954 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:284:43  */
  assign n2955 = trap[12]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:284:34  */
  assign n2956 = n2954 | n2955;
  /* ../../rtl/core/neorv32_cpu_control.vhd:286:27  */
  assign n2958 = n2757[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:286:54  */
  assign n2959 = ~hwtrig_i;
  /* ../../rtl/core/neorv32_cpu_control.vhd:286:40  */
  assign n2960 = n2959 & n2958;
  /* ../../rtl/core/neorv32_cpu_control.vhd:287:40  */
  assign n2961 = n2757[50]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:288:40  */
  assign n2962 = n2757[49]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:289:40  */
  assign n2963 = n2757[32:1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:290:40  */
  assign n2964 = n2757[48:33]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:291:37  */
  assign n2965 = exec[116:86]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:291:51  */
  assign n2967 = {n2965, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:293:29  */
  assign n2969 = n2757[7:3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:293:78  */
  assign n2971 = n2969 == 5'b11100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:294:48  */
  assign n2972 = n2757[32:21]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:286:9  */
  assign n2973 = n2977 ? n2972 : n2805;
  assign n2974 = {n2967, n2962, n2964, n2963, 4'b0100};
  assign n2975 = exec[84:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:286:9  */
  assign n2976 = n2960 ? n2974 : n2975;
  /* ../../rtl/core/neorv32_cpu_control.vhd:286:9  */
  assign n2977 = n2971 & n2960;
  /* ../../rtl/core/neorv32_cpu_control.vhd:286:9  */
  assign n2978 = n2960 ? n2961 : 1'b0;
  assign n2979 = n2976[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:284:9  */
  assign n2980 = n2956 ? 4'b0010 : n2979;
  assign n2981 = n2976[84:4]; // extract
  assign n2982 = exec[84:4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:284:9  */
  assign n2983 = n2956 ? n2982 : n2981;
  /* ../../rtl/core/neorv32_cpu_control.vhd:284:9  */
  assign n2984 = n2956 ? n2805 : n2973;
  /* ../../rtl/core/neorv32_cpu_control.vhd:284:9  */
  assign n2985 = n2956 ? 1'b0 : n2978;
  /* ../../rtl/core/neorv32_cpu_control.vhd:273:7  */
  assign n2987 = n2940 == 4'b0001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:304:25  */
  assign n2990 = csr[63]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:304:51  */
  assign n2991 = trap[61]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:304:36  */
  assign n2992 = n2991 & n2990;
  /* ../../rtl/core/neorv32_cpu_control.vhd:305:36  */
  assign n2993 = csr[94:70]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:305:62  */
  assign n2994 = trap[59:55]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:305:50  */
  assign n2995 = {n2993, n2994};
  /* ../../rtl/core/neorv32_cpu_control.vhd:305:75  */
  assign n2997 = {n2995, 2'b00};
  /* ../../rtl/core/neorv32_cpu_control.vhd:307:36  */
  assign n2998 = csr[94:65]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:307:50  */
  assign n3000 = {n2998, 2'b00};
  /* ../../rtl/core/neorv32_cpu_control.vhd:304:9  */
  assign n3001 = n2992 ? n2997 : n3000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:298:7  */
  assign n3005 = n2940 == 4'b0010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:317:35  */
  assign n3007 = csr[56:26]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:317:49  */
  assign n3009 = {n3007, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:312:7  */
  assign n3013 = n2940 == 4'b0011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:324:34  */
  assign n3014 = alu_add_i[31:1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:324:48  */
  assign n3016 = {n3014, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:332:15  */
  assign n3019 = n2798 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:333:15  */
  assign n3022 = n2798 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:334:15  */
  assign n3024 = n2798 == 3'b011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:335:15  */
  assign n3027 = n2798 == 3'b100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:336:15  */
  assign n3030 = n2798 == 3'b110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:337:15  */
  assign n3033 = n2798 == 3'b111;
  assign n3035 = {n3033, n3030, n3027, n3024, n3022, n3019};
  /* ../../rtl/core/neorv32_cpu_control.vhd:331:13  */
  always @*
    case (n3035)
      6'b100000: n3036 = 3'b111;
      6'b010000: n3036 = 3'b110;
      6'b001000: n3036 = 3'b101;
      6'b000100: n3036 = 3'b011;
      6'b000010: n3036 = 3'b011;
      6'b000001: n3036 = 3'b001;
      default: n3036 = 3'b000;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:342:25  */
  assign n3037 = exec[18:17]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:342:38  */
  assign n3039 = n3037 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_control.vhd:343:27  */
  assign n3041 = n2798 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:343:57  */
  assign n3042 = n2796[5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:343:44  */
  assign n3043 = n3042 & n3041;
  /* ../../rtl/core/neorv32_cpu_control.vhd:343:80  */
  assign n3044 = exec[34]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:343:68  */
  assign n3045 = n3044 & n3043;
  /* ../../rtl/core/neorv32_cpu_control.vhd:342:66  */
  assign n3046 = n3039 | n3045;
  assign n3048 = n2806[118]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:342:13  */
  assign n3049 = n3046 ? 1'b1 : n3048;
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:26  */
  assign n3050 = n2796[5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:30  */
  assign n3051 = ~n3050;
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:51  */
  assign n3053 = n2798 != 3'b001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:37  */
  assign n3054 = n3053 & n3051;
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:82  */
  assign n3056 = n2798 != 3'b101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:68  */
  assign n3057 = n3056 & n3054;
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:26  */
  assign n3058 = n2796[5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:53  */
  assign n3060 = n2798 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:84  */
  assign n3062 = n2797 == 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:70  */
  assign n3063 = n3062 & n3060;
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:112  */
  assign n3065 = n2798 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:143  */
  assign n3067 = n2797 == 7'b0100000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:129  */
  assign n3068 = n3067 & n3065;
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:98  */
  assign n3069 = n3063 | n3068;
  /* ../../rtl/core/neorv32_cpu_control.vhd:350:53  */
  assign n3071 = n2798 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:350:84  */
  assign n3073 = n2797 == 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:350:70  */
  assign n3074 = n3073 & n3071;
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:157  */
  assign n3075 = n3069 | n3074;
  /* ../../rtl/core/neorv32_cpu_control.vhd:350:112  */
  assign n3077 = n2798 == 3'b011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:350:143  */
  assign n3079 = n2797 == 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:350:129  */
  assign n3080 = n3079 & n3077;
  /* ../../rtl/core/neorv32_cpu_control.vhd:350:98  */
  assign n3081 = n3075 | n3080;
  /* ../../rtl/core/neorv32_cpu_control.vhd:351:53  */
  assign n3083 = n2798 == 3'b100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:351:84  */
  assign n3085 = n2797 == 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:351:70  */
  assign n3086 = n3085 & n3083;
  /* ../../rtl/core/neorv32_cpu_control.vhd:350:157  */
  assign n3087 = n3081 | n3086;
  /* ../../rtl/core/neorv32_cpu_control.vhd:351:112  */
  assign n3089 = n2798 == 3'b110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:351:143  */
  assign n3091 = n2797 == 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:351:129  */
  assign n3092 = n3091 & n3089;
  /* ../../rtl/core/neorv32_cpu_control.vhd:351:98  */
  assign n3093 = n3087 | n3092;
  /* ../../rtl/core/neorv32_cpu_control.vhd:352:53  */
  assign n3095 = n2798 == 3'b111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:352:84  */
  assign n3097 = n2797 == 7'b0000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:352:70  */
  assign n3098 = n3097 & n3095;
  /* ../../rtl/core/neorv32_cpu_control.vhd:351:157  */
  assign n3099 = n3093 | n3098;
  /* ../../rtl/core/neorv32_cpu_control.vhd:349:37  */
  assign n3100 = n3099 & n3058;
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:99  */
  assign n3101 = n3057 | n3100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:13  */
  assign n3106 = n3101 ? 4'b0001 : 4'b0101;
  assign n3107 = n2806[98]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:13  */
  assign n3108 = n3101 ? 1'b1 : n3107;
  assign n3109 = n2806[154]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:348:13  */
  assign n3110 = n3101 ? n3109 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:328:11  */
  assign n3112 = n2796 == 7'b0110011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:328:29  */
  assign n3114 = n2796 == 7'b0010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:328:29  */
  assign n3115 = n3112 | n3114;
  /* ../../rtl/core/neorv32_cpu_control.vhd:361:11  */
  assign n3120 = n2796 == 7'b0110111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:367:11  */
  assign n3124 = n2796 == 7'b0010111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:381:45  */
  assign n3125 = exec[9]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:381:34  */
  assign n3126 = ~n3125;
  /* ../../rtl/core/neorv32_cpu_control.vhd:382:41  */
  assign n3127 = exec[9]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:373:11  */
  assign n3130 = n2796 == 7'b0000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:373:30  */
  assign n3132 = n2796 == 7'b0100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:373:30  */
  assign n3133 = n3130 | n3132;
  /* ../../rtl/core/neorv32_cpu_control.vhd:373:47  */
  assign n3135 = n2796 == 7'b0101111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:373:47  */
  assign n3136 = n3133 | n3135;
  /* ../../rtl/core/neorv32_cpu_control.vhd:387:11  */
  assign n3139 = n2796 == 7'b1100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:387:32  */
  assign n3141 = n2796 == 7'b1101111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:387:32  */
  assign n3142 = n3139 | n3141;
  /* ../../rtl/core/neorv32_cpu_control.vhd:387:47  */
  assign n3144 = n2796 == 7'b1100111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:387:47  */
  assign n3145 = n3142 | n3144;
  /* ../../rtl/core/neorv32_cpu_control.vhd:392:42  */
  assign n3146 = exec[16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:392:63  */
  assign n3148 = {n3146, 1'b1};
  /* ../../rtl/core/neorv32_cpu_control.vhd:391:11  */
  assign n3151 = n2796 == 7'b0001111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:396:11  */
  assign n3155 = n2796 == 7'b1010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:401:11  */
  assign n3159 = n2796 == 7'b0001011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:401:31  */
  assign n3161 = n2796 == 7'b0101011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:401:31  */
  assign n3162 = n3159 | n3161;
  /* ../../rtl/core/neorv32_cpu_control.vhd:401:48  */
  assign n3164 = n2796 == 7'b0111011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:401:48  */
  assign n3165 = n3162 | n3164;
  /* ../../rtl/core/neorv32_cpu_control.vhd:401:64  */
  assign n3167 = n2796 == 7'b0011011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:401:64  */
  assign n3168 = n3165 | n3167;
  /* ../../rtl/core/neorv32_cpu_control.vhd:407:26  */
  assign n3170 = n2798 != 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:407:57  */
  assign n3172 = n2798 != 3'b100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:407:43  */
  assign n3173 = n3172 & n3170;
  assign n3175 = n2806[164]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:407:13  */
  assign n3176 = n3173 ? 1'b1 : n3175;
  assign n3178 = {n3168, n3155, n3151, n3145, n3136, n3124, n3120, n3115};
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3179 = 4'b0101;
      8'b01000000: n3179 = 4'b0101;
      8'b00100000: n3179 = 4'b0000;
      8'b00010000: n3179 = 4'b0110;
      8'b00001000: n3179 = 4'b0111;
      8'b00000100: n3179 = 4'b0001;
      8'b00000010: n3179 = 4'b0001;
      8'b00000001: n3179 = n3106;
      default: n3179 = 4'b1001;
    endcase
  assign n3180 = n2806[98]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3181 = n3180;
      8'b01000000: n3181 = n3180;
      8'b00100000: n3181 = n3180;
      8'b00010000: n3181 = n3180;
      8'b00001000: n3181 = n3180;
      8'b00000100: n3181 = 1'b1;
      8'b00000010: n3181 = 1'b1;
      8'b00000001: n3181 = n3108;
      default: n3181 = n3180;
    endcase
  assign n3182 = n2806[117:115]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3183 = n3182;
      8'b01000000: n3183 = n3182;
      8'b00100000: n3183 = n3182;
      8'b00010000: n3183 = n3182;
      8'b00001000: n3183 = n3182;
      8'b00000100: n3183 = 3'b001;
      8'b00000010: n3183 = 3'b100;
      8'b00000001: n3183 = n3036;
      default: n3183 = n3182;
    endcase
  assign n3184 = n2806[118]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3185 = n3184;
      8'b01000000: n3185 = n3184;
      8'b00100000: n3185 = n3184;
      8'b00010000: n3185 = n3184;
      8'b00001000: n3185 = n3184;
      8'b00000100: n3185 = n3184;
      8'b00000010: n3185 = n3184;
      8'b00000001: n3185 = n3049;
      default: n3185 = n3184;
    endcase
  assign n3186 = n2806[154]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3187 = n3186;
      8'b01000000: n3187 = n3186;
      8'b00100000: n3187 = n3186;
      8'b00010000: n3187 = n3186;
      8'b00001000: n3187 = n3186;
      8'b00000100: n3187 = n3186;
      8'b00000010: n3187 = n3186;
      8'b00000001: n3187 = n3110;
      default: n3187 = n3186;
    endcase
  assign n3188 = n2806[155]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3189 = 1'b1;
      8'b01000000: n3189 = n3188;
      8'b00100000: n3189 = n3188;
      8'b00010000: n3189 = n3188;
      8'b00001000: n3189 = n3188;
      8'b00000100: n3189 = n3188;
      8'b00000010: n3189 = n3188;
      8'b00000001: n3189 = n3188;
      default: n3189 = n3188;
    endcase
  assign n3190 = n2806[156]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3191 = n3190;
      8'b01000000: n3191 = 1'b1;
      8'b00100000: n3191 = n3190;
      8'b00010000: n3191 = n3190;
      8'b00001000: n3191 = n3190;
      8'b00000100: n3191 = n3190;
      8'b00000010: n3191 = n3190;
      8'b00000001: n3191 = n3190;
      default: n3191 = n3190;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3192 = n2809;
      8'b01000000: n3192 = n2809;
      8'b00100000: n3192 = n2809;
      8'b00010000: n3192 = n2809;
      8'b00001000: n3192 = n3126;
      8'b00000100: n3192 = n2809;
      8'b00000010: n3192 = n2809;
      8'b00000001: n3192 = n2809;
      default: n3192 = n2809;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3193 = n2812;
      8'b01000000: n3193 = n2812;
      8'b00100000: n3193 = n2812;
      8'b00010000: n3193 = n2812;
      8'b00001000: n3193 = n3127;
      8'b00000100: n3193 = n2812;
      8'b00000010: n3193 = n2812;
      8'b00000001: n3193 = n2812;
      default: n3193 = n2812;
    endcase
  assign n3194 = n2806[164]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3195 = n3194;
      8'b01000000: n3195 = n3194;
      8'b00100000: n3195 = n3194;
      8'b00010000: n3195 = n3194;
      8'b00001000: n3195 = n3194;
      8'b00000100: n3195 = n3194;
      8'b00000010: n3195 = n3194;
      8'b00000001: n3195 = n3194;
      default: n3195 = n3176;
    endcase
  assign n3196 = n2806[263:262]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:325:9  */
  always @*
    case (n3178)
      8'b10000000: n3197 = n3196;
      8'b01000000: n3197 = n3196;
      8'b00100000: n3197 = n3148;
      8'b00010000: n3197 = n3196;
      8'b00001000: n3197 = n3196;
      8'b00000100: n3197 = n3196;
      8'b00000010: n3197 = n3196;
      8'b00000001: n3197 = n3196;
      default: n3197 = n3196;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:322:7  */
  assign n3199 = n2940 == 4'b0100;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3208 = trap[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3210 = 1'b0 | n3208;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3212 = trap[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3213 = n3210 | n3212;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3214 = trap[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3215 = n3213 | n3214;
  /* ../../rtl/core/neorv32_cpu_control.vhd:418:34  */
  assign n3216 = alu_cp_done_i | n3215;
  assign n3218 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:418:9  */
  assign n3219 = n3216 ? 4'b0001 : n3218;
  /* ../../rtl/core/neorv32_cpu_control.vhd:414:7  */
  assign n3221 = n2940 == 4'b0101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:424:29  */
  assign n3223 = 1'b0 | branch_taken;
  assign n3225 = n2806[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:424:9  */
  assign n3226 = n3223 ? 1'b1 : n3225;
  /* ../../rtl/core/neorv32_cpu_control.vhd:428:37  */
  assign n3227 = alu_add_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:428:41  */
  assign n3230 = n3227 & 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:429:37  */
  assign n3231 = alu_add_i[31:1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:429:51  */
  assign n3233 = {n3231, 1'b0};
  assign n3234 = exec[116:85]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:427:9  */
  assign n3235 = branch_taken ? n3233 : n3234;
  /* ../../rtl/core/neorv32_cpu_control.vhd:427:9  */
  assign n3236 = branch_taken ? n3230 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:431:38  */
  assign n3237 = exec[116:86]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:431:52  */
  assign n3239 = {n3237, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:432:37  */
  assign n3240 = exec[6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:422:7  */
  assign n3243 = n2940 == 4'b0110;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3251 = trap[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3253 = 1'b0 | n3251;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3255 = trap[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3256 = n3253 | n3255;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3257 = trap[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3258 = n3256 | n3257;
  /* ../../rtl/core/neorv32_cpu_control.vhd:437:74  */
  assign n3259 = ~n3258;
  /* ../../rtl/core/neorv32_cpu_control.vhd:437:9  */
  assign n3263 = n3259 ? 4'b1000 : 4'b0001;
  assign n3264 = n2806[157]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:437:9  */
  assign n3265 = n3259 ? 1'b1 : n3264;
  /* ../../rtl/core/neorv32_cpu_control.vhd:435:7  */
  assign n3267 = n2940 == 4'b0111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:446:24  */
  assign n3268 = ~lsu_wait_i;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3276 = trap[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3278 = 1'b0 | n3276;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3280 = trap[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3281 = n3278 | n3280;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3282 = trap[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3283 = n3281 | n3282;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3284 = trap[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3285 = n3283 | n3284;
  /* ../../rtl/core/neorv32_cpu_control.vhd:446:31  */
  assign n3286 = n3268 | n3285;
  /* ../../rtl/core/neorv32_cpu_control.vhd:447:37  */
  assign n3287 = ctrl[158]; // extract
  assign n3289 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:446:9  */
  assign n3290 = n3286 ? 4'b0001 : n3289;
  assign n3291 = n2806[98]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:446:9  */
  assign n3292 = n3286 ? n3287 : n3291;
  /* ../../rtl/core/neorv32_cpu_control.vhd:444:7  */
  assign n3294 = n2940 == 4'b1000;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3303 = trap[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3305 = 1'b0 | n3303;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3307 = trap[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3308 = n3305 | n3307;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3309 = trap[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3310 = n3308 | n3309;
  /* ../../rtl/core/neorv32_cpu_control.vhd:454:74  */
  assign n3311 = ~n3310;
  /* ../../rtl/core/neorv32_cpu_control.vhd:455:24  */
  assign n3313 = n2798 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:456:25  */
  assign n3314 = exec[26:24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:457:15  */
  assign n3317 = n3314 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:458:15  */
  assign n3320 = n3314 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:459:15  */
  assign n3323 = n3314 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:460:15  */
  assign n3326 = n3314 == 3'b101;
  assign n3328 = {n3326, n3323, n3320, n3317};
  /* ../../rtl/core/neorv32_cpu_control.vhd:456:13  */
  always @*
    case (n3328)
      4'b1000: n3329 = 4'b1010;
      4'b0100: n3329 = 4'b0011;
      4'b0010: n3329 = 4'b0001;
      4'b0001: n3329 = 4'b0001;
      default: n3329 = 4'b0001;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:456:13  */
  always @*
    case (n3328)
      4'b1000: n3330 = 1'b0;
      4'b0100: n3330 = 1'b0;
      4'b0010: n3330 = 1'b0;
      4'b0001: n3330 = 1'b1;
      default: n3330 = 1'b0;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:456:13  */
  always @*
    case (n3328)
      4'b1000: n3331 = 1'b0;
      4'b0100: n3331 = 1'b0;
      4'b0010: n3331 = 1'b1;
      4'b0001: n3331 = 1'b0;
      default: n3331 = 1'b0;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:463:27  */
  assign n3333 = n2798 != 3'b100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:464:28  */
  assign n3335 = n2798 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:464:59  */
  assign n3337 = n2798 == 3'b101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:464:46  */
  assign n3338 = n3335 | n3337;
  /* ../../rtl/core/neorv32_cpu_control.vhd:464:89  */
  assign n3339 = exec[23:19]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:464:130  */
  assign n3341 = n3339 != 5'b00000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:464:78  */
  assign n3342 = n3338 | n3341;
  /* ../../rtl/core/neorv32_cpu_control.vhd:463:46  */
  assign n3343 = n3342 & n3333;
  assign n3345 = n2806[163]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:463:11  */
  assign n3346 = n3343 ? 1'b1 : n3345;
  /* ../../rtl/core/neorv32_cpu_control.vhd:454:9  */
  assign n3347 = n3353 ? n3329 : 4'b0001;
  assign n3348 = n2806[163]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:455:11  */
  assign n3349 = n3313 ? n3348 : n3346;
  assign n3350 = {n3331, n3330};
  assign n3351 = {1'b0, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:455:11  */
  assign n3352 = n3313 ? n3350 : n3351;
  /* ../../rtl/core/neorv32_cpu_control.vhd:454:9  */
  assign n3353 = n3313 & n3311;
  assign n3354 = n2806[163]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:454:9  */
  assign n3355 = n3311 ? n3349 : n3354;
  assign n3356 = {1'b0, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:454:9  */
  assign n3357 = n3311 ? n3352 : n3356;
  /* ../../rtl/core/neorv32_cpu_control.vhd:451:7  */
  assign n3360 = n2940 == 4'b1001;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3368 = trap[52]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3370 = 1'b0 | n3368;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3372 = trap[51]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3373 = n3370 | n3372;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3374 = trap[50]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3375 = n3373 | n3374;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3376 = trap[49]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3377 = n3375 | n3376;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3378 = trap[48]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3379 = n3377 | n3378;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3380 = trap[47]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3381 = n3379 | n3380;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3382 = trap[46]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3383 = n3381 | n3382;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3384 = trap[45]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3385 = n3383 | n3384;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3386 = trap[44]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3387 = n3385 | n3386;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3388 = trap[43]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3389 = n3387 | n3388;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3390 = trap[42]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3391 = n3389 | n3390;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3392 = trap[41]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3393 = n3391 | n3392;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3394 = trap[40]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3395 = n3393 | n3394;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3396 = trap[39]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3397 = n3395 | n3396;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3398 = trap[38]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3399 = n3397 | n3398;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3400 = trap[37]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3401 = n3399 | n3400;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3402 = trap[36]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3403 = n3401 | n3402;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3404 = trap[35]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3405 = n3403 | n3404;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3406 = trap[34]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3407 = n3405 | n3406;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3408 = trap[33]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3409 = n3407 | n3408;
  /* ../../rtl/core/neorv32_cpu_control.vhd:473:55  */
  assign n3410 = trap[12]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:473:46  */
  assign n3411 = n3409 | n3410;
  assign n3413 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:473:9  */
  assign n3414 = n3411 ? 4'b0001 : n3413;
  assign n3415 = {n3360, n3294, n3267, n3243, n3221, n3199, n3013, n3005, n2987, n2945};
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3416 = n3347;
      10'b0100000000: n3416 = n3290;
      10'b0010000000: n3416 = n3263;
      10'b0001000000: n3416 = 4'b0001;
      10'b0000100000: n3416 = n3219;
      10'b0000010000: n3416 = n3179;
      10'b0000001000: n3416 = 4'b0000;
      10'b0000000100: n3416 = 4'b0000;
      10'b0000000010: n3416 = n2980;
      10'b0000000001: n3416 = 4'b0001;
      default: n3416 = n3414;
    endcase
  assign n3417 = exec[84:4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3418 = n3417;
      10'b0100000000: n3418 = n3417;
      10'b0010000000: n3418 = n3417;
      10'b0001000000: n3418 = n3417;
      10'b0000100000: n3418 = n3417;
      10'b0000010000: n3418 = n3417;
      10'b0000001000: n3418 = n3417;
      10'b0000000100: n3418 = n3417;
      10'b0000000010: n3418 = n2983;
      10'b0000000001: n3418 = n3417;
      default: n3418 = n3417;
    endcase
  assign n3419 = exec[116:85]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3420 = n3419;
      10'b0100000000: n3420 = n3419;
      10'b0010000000: n3420 = n3419;
      10'b0001000000: n3420 = n3235;
      10'b0000100000: n3420 = n3419;
      10'b0000010000: n3420 = n3016;
      10'b0000001000: n3420 = n3009;
      10'b0000000100: n3420 = n3001;
      10'b0000000010: n3420 = n3419;
      10'b0000000001: n3420 = n3419;
      default: n3420 = n3419;
    endcase
  assign n3423 = n2806[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3424 = n3423;
      10'b0100000000: n3424 = n3423;
      10'b0010000000: n3424 = n3423;
      10'b0001000000: n3424 = n3226;
      10'b0000100000: n3424 = n3423;
      10'b0000010000: n3424 = n3423;
      10'b0000001000: n3424 = n3423;
      10'b0000000100: n3424 = n3423;
      10'b0000000010: n3424 = n3423;
      10'b0000000001: n3424 = 1'b1;
      default: n3424 = n3423;
    endcase
  assign n3425 = n2806[97:66]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3426 = n3425;
      10'b0100000000: n3426 = n3425;
      10'b0010000000: n3426 = n3425;
      10'b0001000000: n3426 = n3239;
      10'b0000100000: n3426 = n3425;
      10'b0000010000: n3426 = n3425;
      10'b0000001000: n3426 = n3425;
      10'b0000000100: n3426 = n3425;
      10'b0000000010: n3426 = n3425;
      10'b0000000001: n3426 = n3425;
      default: n3426 = n3425;
    endcase
  assign n3427 = n2806[98]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3428 = 1'b1;
      10'b0100000000: n3428 = n3292;
      10'b0010000000: n3428 = n3427;
      10'b0001000000: n3428 = n3240;
      10'b0000100000: n3428 = alu_cp_done_i;
      10'b0000010000: n3428 = n3181;
      10'b0000001000: n3428 = n3427;
      10'b0000000100: n3428 = n3427;
      10'b0000000010: n3428 = n3427;
      10'b0000000001: n3428 = n3427;
      default: n3428 = n3427;
    endcase
  assign n3429 = n2806[114]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3430 = n3429;
      10'b0100000000: n3430 = n3429;
      10'b0010000000: n3430 = n3429;
      10'b0001000000: n3430 = n3429;
      10'b0000100000: n3430 = n3429;
      10'b0000010000: n3430 = n3429;
      10'b0000001000: n3430 = n3429;
      10'b0000000100: n3430 = n3429;
      10'b0000000010: n3430 = n3429;
      10'b0000000001: n3430 = 1'b1;
      default: n3430 = n3429;
    endcase
  assign n3431 = n2806[117:115]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3432 = n3431;
      10'b0100000000: n3432 = n3431;
      10'b0010000000: n3432 = n3431;
      10'b0001000000: n3432 = n3431;
      10'b0000100000: n3432 = 3'b010;
      10'b0000010000: n3432 = n3183;
      10'b0000001000: n3432 = n3431;
      10'b0000000100: n3432 = n3431;
      10'b0000000010: n3432 = n3431;
      10'b0000000001: n3432 = n3431;
      default: n3432 = n3431;
    endcase
  assign n3433 = n2806[118]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3434 = n3433;
      10'b0100000000: n3434 = n3433;
      10'b0010000000: n3434 = n3433;
      10'b0001000000: n3434 = n3433;
      10'b0000100000: n3434 = n3433;
      10'b0000010000: n3434 = n3185;
      10'b0000001000: n3434 = n3433;
      10'b0000000100: n3434 = n3433;
      10'b0000000010: n3434 = n3433;
      10'b0000000001: n3434 = n3433;
      default: n3434 = n3433;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3435 = n2933;
      10'b0100000000: n3435 = n2933;
      10'b0010000000: n3435 = n2933;
      10'b0001000000: n3435 = n2933;
      10'b0000100000: n3435 = n2933;
      10'b0000010000: n3435 = n2933;
      10'b0000001000: n3435 = n2933;
      10'b0000000100: n3435 = n2933;
      10'b0000000010: n3435 = 1'b1;
      10'b0000000001: n3435 = n2933;
      default: n3435 = n2933;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3436 = n2939;
      10'b0100000000: n3436 = n2939;
      10'b0010000000: n3436 = n2939;
      10'b0001000000: n3436 = n2939;
      10'b0000100000: n3436 = n2939;
      10'b0000010000: n3436 = n2939;
      10'b0000001000: n3436 = n2939;
      10'b0000000100: n3436 = n2939;
      10'b0000000010: n3436 = 1'b1;
      10'b0000000001: n3436 = n2939;
      default: n3436 = n2939;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3437 = n2915;
      10'b0100000000: n3437 = n2915;
      10'b0010000000: n3437 = n2915;
      10'b0001000000: n3437 = n2915;
      10'b0000100000: n3437 = n2915;
      10'b0000010000: n3437 = n2915;
      10'b0000001000: n3437 = n2915;
      10'b0000000100: n3437 = n2915;
      10'b0000000010: n3437 = n2953;
      10'b0000000001: n3437 = n2915;
      default: n3437 = n2915;
    endcase
  assign n3438 = n2806[154]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3439 = n3438;
      10'b0100000000: n3439 = n3438;
      10'b0010000000: n3439 = n3438;
      10'b0001000000: n3439 = n3438;
      10'b0000100000: n3439 = n3438;
      10'b0000010000: n3439 = n3187;
      10'b0000001000: n3439 = n3438;
      10'b0000000100: n3439 = n3438;
      10'b0000000010: n3439 = n3438;
      10'b0000000001: n3439 = n3438;
      default: n3439 = n3438;
    endcase
  assign n3440 = n2806[155]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3441 = n3440;
      10'b0100000000: n3441 = n3440;
      10'b0010000000: n3441 = n3440;
      10'b0001000000: n3441 = n3440;
      10'b0000100000: n3441 = n3440;
      10'b0000010000: n3441 = n3189;
      10'b0000001000: n3441 = n3440;
      10'b0000000100: n3441 = n3440;
      10'b0000000010: n3441 = n3440;
      10'b0000000001: n3441 = n3440;
      default: n3441 = n3440;
    endcase
  assign n3442 = n2806[156]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3443 = n3442;
      10'b0100000000: n3443 = n3442;
      10'b0010000000: n3443 = n3442;
      10'b0001000000: n3443 = n3442;
      10'b0000100000: n3443 = n3442;
      10'b0000010000: n3443 = n3191;
      10'b0000001000: n3443 = n3442;
      10'b0000000100: n3443 = n3442;
      10'b0000000010: n3443 = n3442;
      10'b0000000001: n3443 = n3442;
      default: n3443 = n3442;
    endcase
  assign n3444 = n2806[157]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3445 = n3444;
      10'b0100000000: n3445 = n3444;
      10'b0010000000: n3445 = n3265;
      10'b0001000000: n3445 = n3444;
      10'b0000100000: n3445 = n3444;
      10'b0000010000: n3445 = n3444;
      10'b0000001000: n3445 = n3444;
      10'b0000000100: n3445 = n3444;
      10'b0000000010: n3445 = n3444;
      10'b0000000001: n3445 = n3444;
      default: n3445 = n3444;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3446 = n2809;
      10'b0100000000: n3446 = n2809;
      10'b0010000000: n3446 = n2809;
      10'b0001000000: n3446 = n2809;
      10'b0000100000: n3446 = n2809;
      10'b0000010000: n3446 = n3192;
      10'b0000001000: n3446 = n2809;
      10'b0000000100: n3446 = n2809;
      10'b0000000010: n3446 = n2809;
      10'b0000000001: n3446 = n2809;
      default: n3446 = n2809;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3447 = n2812;
      10'b0100000000: n3447 = n2812;
      10'b0010000000: n3447 = n2812;
      10'b0001000000: n3447 = n2812;
      10'b0000100000: n3447 = n2812;
      10'b0000010000: n3447 = n3193;
      10'b0000001000: n3447 = n2812;
      10'b0000000100: n3447 = n2812;
      10'b0000000010: n3447 = n2812;
      10'b0000000001: n3447 = n2812;
      default: n3447 = n2812;
    endcase
  assign n3448 = n2806[163]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3449 = n3355;
      10'b0100000000: n3449 = n3448;
      10'b0010000000: n3449 = n3448;
      10'b0001000000: n3449 = n3448;
      10'b0000100000: n3449 = n3448;
      10'b0000010000: n3449 = n3448;
      10'b0000001000: n3449 = n3448;
      10'b0000000100: n3449 = n3448;
      10'b0000000010: n3449 = n3448;
      10'b0000000001: n3449 = n3448;
      default: n3449 = n3448;
    endcase
  assign n3450 = n2806[164]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3451 = n3450;
      10'b0100000000: n3451 = n3450;
      10'b0010000000: n3451 = n3450;
      10'b0001000000: n3451 = n3450;
      10'b0000100000: n3451 = n3450;
      10'b0000010000: n3451 = n3195;
      10'b0000001000: n3451 = n3450;
      10'b0000000100: n3451 = n3450;
      10'b0000000010: n3451 = n3450;
      10'b0000000001: n3451 = n3450;
      default: n3451 = n3450;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3452 = n2805;
      10'b0100000000: n3452 = n2805;
      10'b0010000000: n3452 = n2805;
      10'b0001000000: n3452 = n2805;
      10'b0000100000: n3452 = n2805;
      10'b0000010000: n3452 = n2805;
      10'b0000001000: n3452 = n2805;
      10'b0000000100: n3452 = n2805;
      10'b0000000010: n3452 = n2984;
      10'b0000000001: n3452 = n2805;
      default: n3452 = n2805;
    endcase
  assign n3453 = n2806[263:262]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3454 = n3453;
      10'b0100000000: n3454 = n3453;
      10'b0010000000: n3454 = n3453;
      10'b0001000000: n3454 = n3453;
      10'b0000100000: n3454 = n3453;
      10'b0000010000: n3454 = n3197;
      10'b0000001000: n3454 = n3453;
      10'b0000000100: n3454 = n3453;
      10'b0000000010: n3454 = n3453;
      10'b0000000001: n3454 = n3453;
      default: n3454 = n3453;
    endcase
  assign n3457 = n2806[65:1]; // extract
  assign n3460 = n2806[113:99]; // extract
  assign n3466 = n2806[162:160]; // extract
  assign n3467 = n2806[261:177]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3468 = 1'b0;
      10'b0100000000: n3468 = 1'b0;
      10'b0010000000: n3468 = 1'b0;
      10'b0001000000: n3468 = 1'b0;
      10'b0000100000: n3468 = 1'b0;
      10'b0000010000: n3468 = 1'b0;
      10'b0000001000: n3468 = 1'b0;
      10'b0000000100: n3468 = 1'b1;
      10'b0000000010: n3468 = 1'b0;
      10'b0000000001: n3468 = 1'b0;
      default: n3468 = 1'b0;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3469 = 1'b0;
      10'b0100000000: n3469 = 1'b0;
      10'b0010000000: n3469 = 1'b0;
      10'b0001000000: n3469 = 1'b0;
      10'b0000100000: n3469 = 1'b0;
      10'b0000010000: n3469 = 1'b0;
      10'b0000001000: n3469 = 1'b1;
      10'b0000000100: n3469 = 1'b0;
      10'b0000000010: n3469 = 1'b0;
      10'b0000000001: n3469 = 1'b0;
      default: n3469 = 1'b0;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3470 = 1'b0;
      10'b0100000000: n3470 = 1'b0;
      10'b0010000000: n3470 = 1'b0;
      10'b0001000000: n3470 = 1'b0;
      10'b0000100000: n3470 = 1'b0;
      10'b0000010000: n3470 = 1'b0;
      10'b0000001000: n3470 = 1'b0;
      10'b0000000100: n3470 = 1'b0;
      10'b0000000010: n3470 = n2985;
      10'b0000000001: n3470 = 1'b0;
      default: n3470 = 1'b0;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3471 = 1'b0;
      10'b0100000000: n3471 = 1'b0;
      10'b0010000000: n3471 = 1'b0;
      10'b0001000000: n3471 = n3236;
      10'b0000100000: n3471 = 1'b0;
      10'b0000010000: n3471 = 1'b0;
      10'b0000001000: n3471 = 1'b0;
      10'b0000000100: n3471 = 1'b0;
      10'b0000000010: n3471 = 1'b0;
      10'b0000000001: n3471 = 1'b0;
      default: n3471 = 1'b0;
    endcase
  assign n3472 = {1'b0, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:265:5  */
  always @*
    case (n3415)
      10'b1000000000: n3473 = n3357;
      10'b0100000000: n3473 = n3472;
      10'b0010000000: n3473 = n3472;
      10'b0001000000: n3473 = n3472;
      10'b0000100000: n3473 = n3472;
      10'b0000010000: n3473 = n3472;
      10'b0000001000: n3473 = n3472;
      10'b0000000100: n3473 = n3472;
      10'b0000000010: n3473 = n3472;
      10'b0000000001: n3473 = n3472;
      default: n3473 = n3472;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:484:35  */
  assign n3475 = ctrl_nxt[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:485:41  */
  assign n3477 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:485:47  */
  assign n3479 = n3477 == 4'b0001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:485:30  */
  assign n3480 = n3479 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:487:33  */
  assign n3482 = exec[84:54]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:487:47  */
  assign n3484 = {n3482, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:488:34  */
  assign n3485 = exec[116:86]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:488:48  */
  assign n3487 = {n3485, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:489:37  */
  assign n3488 = ctrl[97:67]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:489:51  */
  assign n3490 = {n3488, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:491:31  */
  assign n3491 = ctrl[98]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3499 = trap[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3501 = 1'b0 | n3499;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3503 = trap[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3504 = n3501 | n3503;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3505 = trap[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3506 = n3504 | n3505;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3507 = trap[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3508 = n3506 | n3507;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3509 = trap[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3510 = n3508 | n3509;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3511 = trap[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3512 = n3510 | n3511;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3513 = trap[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3514 = n3512 | n3513;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3515 = trap[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3516 = n3514 | n3515;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3517 = trap[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3518 = n3516 | n3517;
  /* ../../rtl/core/neorv32_cpu_control.vhd:491:45  */
  assign n3519 = ~n3518;
  /* ../../rtl/core/neorv32_cpu_control.vhd:491:40  */
  assign n3520 = n3491 & n3519;
  /* ../../rtl/core/neorv32_cpu_control.vhd:492:33  */
  assign n3521 = exec[23:19]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:493:33  */
  assign n3522 = exec[28:24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:494:33  */
  assign n3523 = exec[15:11]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:495:31  */
  assign n3524 = ctrl[114]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:497:31  */
  assign n3525 = ctrl[117:115]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:498:31  */
  assign n3526 = ctrl[118]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:499:31  */
  assign n3527 = ctrl[119]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:500:31  */
  assign n3528 = ctrl[120]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:501:31  */
  assign n3529 = ctrl[121]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:502:31  */
  assign n3530 = ctrl[153:122]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:503:31  */
  assign n3531 = ctrl[154]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3539 = trap[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3541 = 1'b0 | n3539;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3543 = trap[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3544 = n3541 | n3543;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3545 = trap[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3546 = n3544 | n3545;
  /* ../../rtl/core/neorv32_cpu_control.vhd:503:47  */
  assign n3547 = ~n3546;
  /* ../../rtl/core/neorv32_cpu_control.vhd:503:42  */
  assign n3548 = n3531 & n3547;
  /* ../../rtl/core/neorv32_cpu_control.vhd:504:31  */
  assign n3549 = ctrl[155]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3557 = trap[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3559 = 1'b0 | n3557;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3561 = trap[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3562 = n3559 | n3561;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3563 = trap[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3564 = n3562 | n3563;
  /* ../../rtl/core/neorv32_cpu_control.vhd:504:47  */
  assign n3565 = ~n3564;
  /* ../../rtl/core/neorv32_cpu_control.vhd:504:42  */
  assign n3566 = n3549 & n3565;
  /* ../../rtl/core/neorv32_cpu_control.vhd:505:31  */
  assign n3567 = ctrl[156]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3575 = trap[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3577 = 1'b0 | n3575;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3579 = trap[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3580 = n3577 | n3579;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n3581 = trap[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n3582 = n3580 | n3581;
  /* ../../rtl/core/neorv32_cpu_control.vhd:505:47  */
  assign n3583 = ~n3582;
  /* ../../rtl/core/neorv32_cpu_control.vhd:505:42  */
  assign n3584 = n3567 & n3583;
  /* ../../rtl/core/neorv32_cpu_control.vhd:507:31  */
  assign n3585 = ctrl[157]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:508:31  */
  assign n3586 = ctrl[158]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:509:31  */
  assign n3587 = ctrl[159]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:510:41  */
  assign n3589 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:510:47  */
  assign n3591 = n3589 == 4'b0111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:510:30  */
  assign n3592 = n3591 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:511:41  */
  assign n3595 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:511:47  */
  assign n3597 = n3595 == 4'b1000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:511:30  */
  assign n3598 = n3597 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:512:30  */
  assign n3600 = csr[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:512:52  */
  assign n3601 = csr[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:512:42  */
  assign n3602 = n3601 ? n3600 : n3603;
  /* ../../rtl/core/neorv32_cpu_control.vhd:512:81  */
  assign n3603 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:514:31  */
  assign n3604 = ctrl[163]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:515:31  */
  assign n3605 = ctrl[164]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:516:31  */
  assign n3606 = ctrl[176:165]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:521:33  */
  assign n3607 = exec[18:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:522:33  */
  assign n3608 = exec[35:24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:523:33  */
  assign n3609 = exec[10:4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:524:31  */
  assign n3610 = exec[51:36]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:526:30  */
  assign n3611 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:527:31  */
  assign n3612 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:528:31  */
  assign n3613 = trap[12]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:529:37  */
  assign n3614 = debug_ctrl[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:530:31  */
  assign n3615 = ctrl[263:262]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:535:53  */
  assign n3617 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:535:59  */
  assign n3619 = n3617 == 4'b1010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:535:42  */
  assign n3620 = n3619 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:537:53  */
  assign n3624 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:537:59  */
  assign n3626 = n3624 == 4'b0100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:537:42  */
  assign n3627 = n3626 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:538:53  */
  assign n3630 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:538:59  */
  assign n3632 = n3630 == 4'b0100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:538:82  */
  assign n3633 = exec[52]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:538:72  */
  assign n3634 = n3633 & n3632;
  /* ../../rtl/core/neorv32_cpu_control.vhd:538:42  */
  assign n3635 = n3634 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:539:53  */
  assign n3638 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:539:59  */
  assign n3640 = n3638 == 4'b0001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:539:89  */
  assign n3641 = n2757[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:539:95  */
  assign n3642 = ~n3641;
  /* ../../rtl/core/neorv32_cpu_control.vhd:539:73  */
  assign n3643 = n3642 & n3640;
  /* ../../rtl/core/neorv32_cpu_control.vhd:539:42  */
  assign n3644 = n3643 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:540:53  */
  assign n3647 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:540:59  */
  assign n3649 = n3647 == 4'b0101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:540:42  */
  assign n3650 = n3649 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:541:53  */
  assign n3653 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:541:59  */
  assign n3655 = n3653 == 4'b0110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:541:42  */
  assign n3656 = n3655 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:542:53  */
  assign n3659 = ctrl[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:542:81  */
  assign n3660 = exec[10:6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:542:94  */
  assign n3662 = n3660 != 5'b00011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:542:69  */
  assign n3663 = n3662 & n3659;
  /* ../../rtl/core/neorv32_cpu_control.vhd:542:42  */
  assign n3664 = n3663 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:543:53  */
  assign n3667 = ctrl[157]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:543:78  */
  assign n3668 = ctrl[158]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:543:68  */
  assign n3669 = n3668 & n3667;
  /* ../../rtl/core/neorv32_cpu_control.vhd:543:42  */
  assign n3670 = n3669 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:544:53  */
  assign n3673 = ctrl[157]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:544:78  */
  assign n3674 = ctrl[159]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:544:68  */
  assign n3675 = n3674 & n3673;
  /* ../../rtl/core/neorv32_cpu_control.vhd:544:42  */
  assign n3676 = n3675 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:545:53  */
  assign n3679 = ctrl[157]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:545:61  */
  assign n3680 = ~n3679;
  /* ../../rtl/core/neorv32_cpu_control.vhd:545:78  */
  assign n3681 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:545:84  */
  assign n3683 = n3681 == 4'b1000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:545:68  */
  assign n3684 = n3683 & n3680;
  /* ../../rtl/core/neorv32_cpu_control.vhd:545:42  */
  assign n3685 = n3684 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:559:15  */
  assign n3688 = ctrl[176:165]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:562:7  */
  assign n3692 = n3688 == 12'b000000000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:562:25  */
  assign n3694 = n3688 == 12'b000000000010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:562:25  */
  assign n3695 = n3692 | n3694;
  /* ../../rtl/core/neorv32_cpu_control.vhd:562:37  */
  assign n3697 = n3688 == 12'b000000000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:562:37  */
  assign n3698 = n3695 | n3697;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:7  */
  assign n3701 = n3688 == 12'b001100000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:32  */
  assign n3703 = n3688 == 12'b001100010000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:32  */
  assign n3704 = n3701 | n3703;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:50  */
  assign n3706 = n3688 == 12'b001100000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:50  */
  assign n3707 = n3704 | n3706;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:66  */
  assign n3709 = n3688 == 12'b001100000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:66  */
  assign n3710 = n3707 | n3709;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:81  */
  assign n3712 = n3688 == 12'b001100000101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:81  */
  assign n3713 = n3710 | n3712;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:96  */
  assign n3715 = n3688 == 12'b111100010100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:96  */
  assign n3716 = n3713 | n3715;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:115  */
  assign n3718 = n3688 == 12'b001101000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:566:115  */
  assign n3719 = n3716 | n3718;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:32  */
  assign n3721 = n3688 == 12'b001101000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:32  */
  assign n3722 = n3719 | n3721;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:50  */
  assign n3724 = n3688 == 12'b001101000010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:50  */
  assign n3725 = n3722 | n3724;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:66  */
  assign n3727 = n3688 == 12'b001101000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:66  */
  assign n3728 = n3725 | n3727;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:81  */
  assign n3730 = n3688 == 12'b001101000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:81  */
  assign n3731 = n3728 | n3730;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:96  */
  assign n3733 = n3688 == 12'b111100010101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:96  */
  assign n3734 = n3731 | n3733;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:115  */
  assign n3736 = n3688 == 12'b001100100000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:567:115  */
  assign n3737 = n3734 | n3736;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:32  */
  assign n3739 = n3688 == 12'b111100010001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:32  */
  assign n3740 = n3737 | n3739;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:50  */
  assign n3742 = n3688 == 12'b111100010010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:50  */
  assign n3743 = n3740 | n3742;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:66  */
  assign n3745 = n3688 == 12'b111100010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:66  */
  assign n3746 = n3743 | n3745;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:81  */
  assign n3748 = n3688 == 12'b111111000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:81  */
  assign n3749 = n3746 | n3748;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:96  */
  assign n3751 = n3688 == 12'b111111000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:568:96  */
  assign n3752 = n3749 | n3751;
  /* ../../rtl/core/neorv32_cpu_control.vhd:572:7  */
  assign n3756 = n3688 == 12'b001100000110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:572:29  */
  assign n3758 = n3688 == 12'b001100001010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:572:29  */
  assign n3759 = n3756 | n3758;
  /* ../../rtl/core/neorv32_cpu_control.vhd:572:45  */
  assign n3761 = n3688 == 12'b001100011010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:572:45  */
  assign n3762 = n3759 | n3761;
  /* ../../rtl/core/neorv32_cpu_control.vhd:576:7  */
  assign n3766 = n3688 == 12'b001110100000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:576:28  */
  assign n3768 = n3688 == 12'b001110100001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:576:28  */
  assign n3769 = n3766 | n3768;
  /* ../../rtl/core/neorv32_cpu_control.vhd:576:46  */
  assign n3771 = n3688 == 12'b001110100010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:576:46  */
  assign n3772 = n3769 | n3771;
  /* ../../rtl/core/neorv32_cpu_control.vhd:576:64  */
  assign n3774 = n3688 == 12'b001110100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:576:64  */
  assign n3775 = n3772 | n3774;
  /* ../../rtl/core/neorv32_cpu_control.vhd:576:82  */
  assign n3777 = n3688 == 12'b001110110000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:576:82  */
  assign n3778 = n3775 | n3777;
  /* ../../rtl/core/neorv32_cpu_control.vhd:577:28  */
  assign n3780 = n3688 == 12'b001110110001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:577:28  */
  assign n3781 = n3778 | n3780;
  /* ../../rtl/core/neorv32_cpu_control.vhd:577:46  */
  assign n3783 = n3688 == 12'b001110110010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:577:46  */
  assign n3784 = n3781 | n3783;
  /* ../../rtl/core/neorv32_cpu_control.vhd:577:64  */
  assign n3786 = n3688 == 12'b001110110011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:577:64  */
  assign n3787 = n3784 | n3786;
  /* ../../rtl/core/neorv32_cpu_control.vhd:577:82  */
  assign n3789 = n3688 == 12'b001110110100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:577:82  */
  assign n3790 = n3787 | n3789;
  /* ../../rtl/core/neorv32_cpu_control.vhd:578:28  */
  assign n3792 = n3688 == 12'b001110110101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:578:28  */
  assign n3793 = n3790 | n3792;
  /* ../../rtl/core/neorv32_cpu_control.vhd:578:46  */
  assign n3795 = n3688 == 12'b001110110110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:578:46  */
  assign n3796 = n3793 | n3795;
  /* ../../rtl/core/neorv32_cpu_control.vhd:578:64  */
  assign n3798 = n3688 == 12'b001110110111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:578:64  */
  assign n3799 = n3796 | n3798;
  /* ../../rtl/core/neorv32_cpu_control.vhd:578:82  */
  assign n3801 = n3688 == 12'b001110111000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:578:82  */
  assign n3802 = n3799 | n3801;
  /* ../../rtl/core/neorv32_cpu_control.vhd:579:28  */
  assign n3804 = n3688 == 12'b001110111001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:579:28  */
  assign n3805 = n3802 | n3804;
  /* ../../rtl/core/neorv32_cpu_control.vhd:579:46  */
  assign n3807 = n3688 == 12'b001110111010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:579:46  */
  assign n3808 = n3805 | n3807;
  /* ../../rtl/core/neorv32_cpu_control.vhd:579:64  */
  assign n3810 = n3688 == 12'b001110111011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:579:64  */
  assign n3811 = n3808 | n3810;
  /* ../../rtl/core/neorv32_cpu_control.vhd:579:82  */
  assign n3813 = n3688 == 12'b001110111100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:579:82  */
  assign n3814 = n3811 | n3813;
  /* ../../rtl/core/neorv32_cpu_control.vhd:580:28  */
  assign n3816 = n3688 == 12'b001110111101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:580:28  */
  assign n3817 = n3814 | n3816;
  /* ../../rtl/core/neorv32_cpu_control.vhd:580:46  */
  assign n3819 = n3688 == 12'b001110111110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:580:46  */
  assign n3820 = n3817 | n3819;
  /* ../../rtl/core/neorv32_cpu_control.vhd:580:64  */
  assign n3822 = n3688 == 12'b001110111111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:580:64  */
  assign n3823 = n3820 | n3822;
  /* ../../rtl/core/neorv32_cpu_control.vhd:583:7  */
  assign n3827 = n3688 == 12'b110000000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:583:33  */
  assign n3829 = n3688 == 12'b110000000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:583:33  */
  assign n3830 = n3827 | n3829;
  /* ../../rtl/core/neorv32_cpu_control.vhd:583:56  */
  assign n3832 = n3688 == 12'b110000000101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:583:56  */
  assign n3833 = n3830 | n3832;
  /* ../../rtl/core/neorv32_cpu_control.vhd:583:79  */
  assign n3835 = n3688 == 12'b110000000110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:583:79  */
  assign n3836 = n3833 | n3835;
  /* ../../rtl/core/neorv32_cpu_control.vhd:583:102  */
  assign n3838 = n3688 == 12'b110000000111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:583:102  */
  assign n3839 = n3836 | n3838;
  /* ../../rtl/core/neorv32_cpu_control.vhd:584:33  */
  assign n3841 = n3688 == 12'b110000001000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:584:33  */
  assign n3842 = n3839 | n3841;
  /* ../../rtl/core/neorv32_cpu_control.vhd:584:56  */
  assign n3844 = n3688 == 12'b110000001001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:584:56  */
  assign n3845 = n3842 | n3844;
  /* ../../rtl/core/neorv32_cpu_control.vhd:584:79  */
  assign n3847 = n3688 == 12'b110000001010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:584:79  */
  assign n3848 = n3845 | n3847;
  /* ../../rtl/core/neorv32_cpu_control.vhd:584:102  */
  assign n3850 = n3688 == 12'b110000001011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:584:102  */
  assign n3851 = n3848 | n3850;
  /* ../../rtl/core/neorv32_cpu_control.vhd:585:33  */
  assign n3853 = n3688 == 12'b110000001100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:585:33  */
  assign n3854 = n3851 | n3853;
  /* ../../rtl/core/neorv32_cpu_control.vhd:585:56  */
  assign n3856 = n3688 == 12'b110000001101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:585:56  */
  assign n3857 = n3854 | n3856;
  /* ../../rtl/core/neorv32_cpu_control.vhd:585:79  */
  assign n3859 = n3688 == 12'b110000001110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:585:79  */
  assign n3860 = n3857 | n3859;
  /* ../../rtl/core/neorv32_cpu_control.vhd:585:102  */
  assign n3862 = n3688 == 12'b110000001111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:585:102  */
  assign n3863 = n3860 | n3862;
  /* ../../rtl/core/neorv32_cpu_control.vhd:586:33  */
  assign n3865 = n3688 == 12'b110000010000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:586:33  */
  assign n3866 = n3863 | n3865;
  /* ../../rtl/core/neorv32_cpu_control.vhd:586:56  */
  assign n3868 = n3688 == 12'b110000010001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:586:56  */
  assign n3869 = n3866 | n3868;
  /* ../../rtl/core/neorv32_cpu_control.vhd:586:79  */
  assign n3871 = n3688 == 12'b110000010010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:586:79  */
  assign n3872 = n3869 | n3871;
  /* ../../rtl/core/neorv32_cpu_control.vhd:586:102  */
  assign n3874 = n3688 == 12'b110000010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:586:102  */
  assign n3875 = n3872 | n3874;
  /* ../../rtl/core/neorv32_cpu_control.vhd:587:33  */
  assign n3877 = n3688 == 12'b110000010100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:587:33  */
  assign n3878 = n3875 | n3877;
  /* ../../rtl/core/neorv32_cpu_control.vhd:587:56  */
  assign n3880 = n3688 == 12'b110000010101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:587:56  */
  assign n3881 = n3878 | n3880;
  /* ../../rtl/core/neorv32_cpu_control.vhd:587:79  */
  assign n3883 = n3688 == 12'b110000010110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:587:79  */
  assign n3884 = n3881 | n3883;
  /* ../../rtl/core/neorv32_cpu_control.vhd:587:102  */
  assign n3886 = n3688 == 12'b110000010111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:587:102  */
  assign n3887 = n3884 | n3886;
  /* ../../rtl/core/neorv32_cpu_control.vhd:588:33  */
  assign n3889 = n3688 == 12'b110000011000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:588:33  */
  assign n3890 = n3887 | n3889;
  /* ../../rtl/core/neorv32_cpu_control.vhd:588:56  */
  assign n3892 = n3688 == 12'b110000011001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:588:56  */
  assign n3893 = n3890 | n3892;
  /* ../../rtl/core/neorv32_cpu_control.vhd:588:79  */
  assign n3895 = n3688 == 12'b110000011010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:588:79  */
  assign n3896 = n3893 | n3895;
  /* ../../rtl/core/neorv32_cpu_control.vhd:588:102  */
  assign n3898 = n3688 == 12'b110000011011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:588:102  */
  assign n3899 = n3896 | n3898;
  /* ../../rtl/core/neorv32_cpu_control.vhd:589:33  */
  assign n3901 = n3688 == 12'b110000011100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:589:33  */
  assign n3902 = n3899 | n3901;
  /* ../../rtl/core/neorv32_cpu_control.vhd:589:56  */
  assign n3904 = n3688 == 12'b110000011101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:589:56  */
  assign n3905 = n3902 | n3904;
  /* ../../rtl/core/neorv32_cpu_control.vhd:589:79  */
  assign n3907 = n3688 == 12'b110000011110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:589:79  */
  assign n3908 = n3905 | n3907;
  /* ../../rtl/core/neorv32_cpu_control.vhd:589:102  */
  assign n3910 = n3688 == 12'b110000011111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:589:102  */
  assign n3911 = n3908 | n3910;
  /* ../../rtl/core/neorv32_cpu_control.vhd:590:33  */
  assign n3913 = n3688 == 12'b110010000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:590:33  */
  assign n3914 = n3911 | n3913;
  /* ../../rtl/core/neorv32_cpu_control.vhd:591:33  */
  assign n3916 = n3688 == 12'b110010000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:591:33  */
  assign n3917 = n3914 | n3916;
  /* ../../rtl/core/neorv32_cpu_control.vhd:591:56  */
  assign n3919 = n3688 == 12'b110010000101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:591:56  */
  assign n3920 = n3917 | n3919;
  /* ../../rtl/core/neorv32_cpu_control.vhd:591:79  */
  assign n3922 = n3688 == 12'b110010000110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:591:79  */
  assign n3923 = n3920 | n3922;
  /* ../../rtl/core/neorv32_cpu_control.vhd:591:102  */
  assign n3925 = n3688 == 12'b110010000111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:591:102  */
  assign n3926 = n3923 | n3925;
  /* ../../rtl/core/neorv32_cpu_control.vhd:592:33  */
  assign n3928 = n3688 == 12'b110010001000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:592:33  */
  assign n3929 = n3926 | n3928;
  /* ../../rtl/core/neorv32_cpu_control.vhd:592:56  */
  assign n3931 = n3688 == 12'b110010001001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:592:56  */
  assign n3932 = n3929 | n3931;
  /* ../../rtl/core/neorv32_cpu_control.vhd:592:79  */
  assign n3934 = n3688 == 12'b110010001010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:592:79  */
  assign n3935 = n3932 | n3934;
  /* ../../rtl/core/neorv32_cpu_control.vhd:592:102  */
  assign n3937 = n3688 == 12'b110010001011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:592:102  */
  assign n3938 = n3935 | n3937;
  /* ../../rtl/core/neorv32_cpu_control.vhd:593:33  */
  assign n3940 = n3688 == 12'b110010001100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:593:33  */
  assign n3941 = n3938 | n3940;
  /* ../../rtl/core/neorv32_cpu_control.vhd:593:56  */
  assign n3943 = n3688 == 12'b110010001101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:593:56  */
  assign n3944 = n3941 | n3943;
  /* ../../rtl/core/neorv32_cpu_control.vhd:593:79  */
  assign n3946 = n3688 == 12'b110010001110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:593:79  */
  assign n3947 = n3944 | n3946;
  /* ../../rtl/core/neorv32_cpu_control.vhd:593:102  */
  assign n3949 = n3688 == 12'b110010001111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:593:102  */
  assign n3950 = n3947 | n3949;
  /* ../../rtl/core/neorv32_cpu_control.vhd:594:33  */
  assign n3952 = n3688 == 12'b110010010000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:594:33  */
  assign n3953 = n3950 | n3952;
  /* ../../rtl/core/neorv32_cpu_control.vhd:594:56  */
  assign n3955 = n3688 == 12'b110010010001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:594:56  */
  assign n3956 = n3953 | n3955;
  /* ../../rtl/core/neorv32_cpu_control.vhd:594:79  */
  assign n3958 = n3688 == 12'b110010010010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:594:79  */
  assign n3959 = n3956 | n3958;
  /* ../../rtl/core/neorv32_cpu_control.vhd:594:102  */
  assign n3961 = n3688 == 12'b110010010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:594:102  */
  assign n3962 = n3959 | n3961;
  /* ../../rtl/core/neorv32_cpu_control.vhd:595:33  */
  assign n3964 = n3688 == 12'b110010010100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:595:33  */
  assign n3965 = n3962 | n3964;
  /* ../../rtl/core/neorv32_cpu_control.vhd:595:56  */
  assign n3967 = n3688 == 12'b110010010101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:595:56  */
  assign n3968 = n3965 | n3967;
  /* ../../rtl/core/neorv32_cpu_control.vhd:595:79  */
  assign n3970 = n3688 == 12'b110010010110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:595:79  */
  assign n3971 = n3968 | n3970;
  /* ../../rtl/core/neorv32_cpu_control.vhd:595:102  */
  assign n3973 = n3688 == 12'b110010010111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:595:102  */
  assign n3974 = n3971 | n3973;
  /* ../../rtl/core/neorv32_cpu_control.vhd:596:33  */
  assign n3976 = n3688 == 12'b110010011000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:596:33  */
  assign n3977 = n3974 | n3976;
  /* ../../rtl/core/neorv32_cpu_control.vhd:596:56  */
  assign n3979 = n3688 == 12'b110010011001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:596:56  */
  assign n3980 = n3977 | n3979;
  /* ../../rtl/core/neorv32_cpu_control.vhd:596:79  */
  assign n3982 = n3688 == 12'b110010011010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:596:79  */
  assign n3983 = n3980 | n3982;
  /* ../../rtl/core/neorv32_cpu_control.vhd:596:102  */
  assign n3985 = n3688 == 12'b110010011011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:596:102  */
  assign n3986 = n3983 | n3985;
  /* ../../rtl/core/neorv32_cpu_control.vhd:597:33  */
  assign n3988 = n3688 == 12'b110010011100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:597:33  */
  assign n3989 = n3986 | n3988;
  /* ../../rtl/core/neorv32_cpu_control.vhd:597:56  */
  assign n3991 = n3688 == 12'b110010011101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:597:56  */
  assign n3992 = n3989 | n3991;
  /* ../../rtl/core/neorv32_cpu_control.vhd:597:79  */
  assign n3994 = n3688 == 12'b110010011110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:597:79  */
  assign n3995 = n3992 | n3994;
  /* ../../rtl/core/neorv32_cpu_control.vhd:597:102  */
  assign n3997 = n3688 == 12'b110010011111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:597:102  */
  assign n3998 = n3995 | n3997;
  /* ../../rtl/core/neorv32_cpu_control.vhd:598:33  */
  assign n4000 = n3688 == 12'b101100000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:598:33  */
  assign n4001 = n3998 | n4000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:599:33  */
  assign n4003 = n3688 == 12'b101100000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:599:33  */
  assign n4004 = n4001 | n4003;
  /* ../../rtl/core/neorv32_cpu_control.vhd:599:56  */
  assign n4006 = n3688 == 12'b101100000101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:599:56  */
  assign n4007 = n4004 | n4006;
  /* ../../rtl/core/neorv32_cpu_control.vhd:599:79  */
  assign n4009 = n3688 == 12'b101100000110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:599:79  */
  assign n4010 = n4007 | n4009;
  /* ../../rtl/core/neorv32_cpu_control.vhd:599:102  */
  assign n4012 = n3688 == 12'b101100000111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:599:102  */
  assign n4013 = n4010 | n4012;
  /* ../../rtl/core/neorv32_cpu_control.vhd:600:33  */
  assign n4015 = n3688 == 12'b101100001000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:600:33  */
  assign n4016 = n4013 | n4015;
  /* ../../rtl/core/neorv32_cpu_control.vhd:600:56  */
  assign n4018 = n3688 == 12'b101100001001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:600:56  */
  assign n4019 = n4016 | n4018;
  /* ../../rtl/core/neorv32_cpu_control.vhd:600:79  */
  assign n4021 = n3688 == 12'b101100001010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:600:79  */
  assign n4022 = n4019 | n4021;
  /* ../../rtl/core/neorv32_cpu_control.vhd:600:102  */
  assign n4024 = n3688 == 12'b101100001011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:600:102  */
  assign n4025 = n4022 | n4024;
  /* ../../rtl/core/neorv32_cpu_control.vhd:601:33  */
  assign n4027 = n3688 == 12'b101100001100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:601:33  */
  assign n4028 = n4025 | n4027;
  /* ../../rtl/core/neorv32_cpu_control.vhd:601:56  */
  assign n4030 = n3688 == 12'b101100001101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:601:56  */
  assign n4031 = n4028 | n4030;
  /* ../../rtl/core/neorv32_cpu_control.vhd:601:79  */
  assign n4033 = n3688 == 12'b101100001110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:601:79  */
  assign n4034 = n4031 | n4033;
  /* ../../rtl/core/neorv32_cpu_control.vhd:601:102  */
  assign n4036 = n3688 == 12'b101100001111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:601:102  */
  assign n4037 = n4034 | n4036;
  /* ../../rtl/core/neorv32_cpu_control.vhd:602:33  */
  assign n4039 = n3688 == 12'b101100010000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:602:33  */
  assign n4040 = n4037 | n4039;
  /* ../../rtl/core/neorv32_cpu_control.vhd:602:56  */
  assign n4042 = n3688 == 12'b101100010001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:602:56  */
  assign n4043 = n4040 | n4042;
  /* ../../rtl/core/neorv32_cpu_control.vhd:602:79  */
  assign n4045 = n3688 == 12'b101100010010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:602:79  */
  assign n4046 = n4043 | n4045;
  /* ../../rtl/core/neorv32_cpu_control.vhd:602:102  */
  assign n4048 = n3688 == 12'b101100010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:602:102  */
  assign n4049 = n4046 | n4048;
  /* ../../rtl/core/neorv32_cpu_control.vhd:603:33  */
  assign n4051 = n3688 == 12'b101100010100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:603:33  */
  assign n4052 = n4049 | n4051;
  /* ../../rtl/core/neorv32_cpu_control.vhd:603:56  */
  assign n4054 = n3688 == 12'b101100010101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:603:56  */
  assign n4055 = n4052 | n4054;
  /* ../../rtl/core/neorv32_cpu_control.vhd:603:79  */
  assign n4057 = n3688 == 12'b101100010110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:603:79  */
  assign n4058 = n4055 | n4057;
  /* ../../rtl/core/neorv32_cpu_control.vhd:603:102  */
  assign n4060 = n3688 == 12'b101100010111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:603:102  */
  assign n4061 = n4058 | n4060;
  /* ../../rtl/core/neorv32_cpu_control.vhd:604:33  */
  assign n4063 = n3688 == 12'b101100011000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:604:33  */
  assign n4064 = n4061 | n4063;
  /* ../../rtl/core/neorv32_cpu_control.vhd:604:56  */
  assign n4066 = n3688 == 12'b101100011001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:604:56  */
  assign n4067 = n4064 | n4066;
  /* ../../rtl/core/neorv32_cpu_control.vhd:604:79  */
  assign n4069 = n3688 == 12'b101100011010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:604:79  */
  assign n4070 = n4067 | n4069;
  /* ../../rtl/core/neorv32_cpu_control.vhd:604:102  */
  assign n4072 = n3688 == 12'b101100011011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:604:102  */
  assign n4073 = n4070 | n4072;
  /* ../../rtl/core/neorv32_cpu_control.vhd:605:33  */
  assign n4075 = n3688 == 12'b101100011100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:605:33  */
  assign n4076 = n4073 | n4075;
  /* ../../rtl/core/neorv32_cpu_control.vhd:605:56  */
  assign n4078 = n3688 == 12'b101100011101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:605:56  */
  assign n4079 = n4076 | n4078;
  /* ../../rtl/core/neorv32_cpu_control.vhd:605:79  */
  assign n4081 = n3688 == 12'b101100011110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:605:79  */
  assign n4082 = n4079 | n4081;
  /* ../../rtl/core/neorv32_cpu_control.vhd:605:102  */
  assign n4084 = n3688 == 12'b101100011111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:605:102  */
  assign n4085 = n4082 | n4084;
  /* ../../rtl/core/neorv32_cpu_control.vhd:606:33  */
  assign n4087 = n3688 == 12'b101110000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:606:33  */
  assign n4088 = n4085 | n4087;
  /* ../../rtl/core/neorv32_cpu_control.vhd:607:33  */
  assign n4090 = n3688 == 12'b101110000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:607:33  */
  assign n4091 = n4088 | n4090;
  /* ../../rtl/core/neorv32_cpu_control.vhd:607:56  */
  assign n4093 = n3688 == 12'b101110000101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:607:56  */
  assign n4094 = n4091 | n4093;
  /* ../../rtl/core/neorv32_cpu_control.vhd:607:79  */
  assign n4096 = n3688 == 12'b101110000110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:607:79  */
  assign n4097 = n4094 | n4096;
  /* ../../rtl/core/neorv32_cpu_control.vhd:607:102  */
  assign n4099 = n3688 == 12'b101110000111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:607:102  */
  assign n4100 = n4097 | n4099;
  /* ../../rtl/core/neorv32_cpu_control.vhd:608:33  */
  assign n4102 = n3688 == 12'b101110001000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:608:33  */
  assign n4103 = n4100 | n4102;
  /* ../../rtl/core/neorv32_cpu_control.vhd:608:56  */
  assign n4105 = n3688 == 12'b101110001001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:608:56  */
  assign n4106 = n4103 | n4105;
  /* ../../rtl/core/neorv32_cpu_control.vhd:608:79  */
  assign n4108 = n3688 == 12'b101110001010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:608:79  */
  assign n4109 = n4106 | n4108;
  /* ../../rtl/core/neorv32_cpu_control.vhd:608:102  */
  assign n4111 = n3688 == 12'b101110001011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:608:102  */
  assign n4112 = n4109 | n4111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:609:33  */
  assign n4114 = n3688 == 12'b101110001100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:609:33  */
  assign n4115 = n4112 | n4114;
  /* ../../rtl/core/neorv32_cpu_control.vhd:609:56  */
  assign n4117 = n3688 == 12'b101110001101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:609:56  */
  assign n4118 = n4115 | n4117;
  /* ../../rtl/core/neorv32_cpu_control.vhd:609:79  */
  assign n4120 = n3688 == 12'b101110001110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:609:79  */
  assign n4121 = n4118 | n4120;
  /* ../../rtl/core/neorv32_cpu_control.vhd:609:102  */
  assign n4123 = n3688 == 12'b101110001111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:609:102  */
  assign n4124 = n4121 | n4123;
  /* ../../rtl/core/neorv32_cpu_control.vhd:610:33  */
  assign n4126 = n3688 == 12'b101110010000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:610:33  */
  assign n4127 = n4124 | n4126;
  /* ../../rtl/core/neorv32_cpu_control.vhd:610:56  */
  assign n4129 = n3688 == 12'b101110010001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:610:56  */
  assign n4130 = n4127 | n4129;
  /* ../../rtl/core/neorv32_cpu_control.vhd:610:79  */
  assign n4132 = n3688 == 12'b101110010010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:610:79  */
  assign n4133 = n4130 | n4132;
  /* ../../rtl/core/neorv32_cpu_control.vhd:610:102  */
  assign n4135 = n3688 == 12'b101110010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:610:102  */
  assign n4136 = n4133 | n4135;
  /* ../../rtl/core/neorv32_cpu_control.vhd:611:33  */
  assign n4138 = n3688 == 12'b101110010100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:611:33  */
  assign n4139 = n4136 | n4138;
  /* ../../rtl/core/neorv32_cpu_control.vhd:611:56  */
  assign n4141 = n3688 == 12'b101110010101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:611:56  */
  assign n4142 = n4139 | n4141;
  /* ../../rtl/core/neorv32_cpu_control.vhd:611:79  */
  assign n4144 = n3688 == 12'b101110010110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:611:79  */
  assign n4145 = n4142 | n4144;
  /* ../../rtl/core/neorv32_cpu_control.vhd:611:102  */
  assign n4147 = n3688 == 12'b101110010111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:611:102  */
  assign n4148 = n4145 | n4147;
  /* ../../rtl/core/neorv32_cpu_control.vhd:612:33  */
  assign n4150 = n3688 == 12'b101110011000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:612:33  */
  assign n4151 = n4148 | n4150;
  /* ../../rtl/core/neorv32_cpu_control.vhd:612:56  */
  assign n4153 = n3688 == 12'b101110011001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:612:56  */
  assign n4154 = n4151 | n4153;
  /* ../../rtl/core/neorv32_cpu_control.vhd:612:79  */
  assign n4156 = n3688 == 12'b101110011010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:612:79  */
  assign n4157 = n4154 | n4156;
  /* ../../rtl/core/neorv32_cpu_control.vhd:612:102  */
  assign n4159 = n3688 == 12'b101110011011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:612:102  */
  assign n4160 = n4157 | n4159;
  /* ../../rtl/core/neorv32_cpu_control.vhd:613:33  */
  assign n4162 = n3688 == 12'b101110011100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:613:33  */
  assign n4163 = n4160 | n4162;
  /* ../../rtl/core/neorv32_cpu_control.vhd:613:56  */
  assign n4165 = n3688 == 12'b101110011101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:613:56  */
  assign n4166 = n4163 | n4165;
  /* ../../rtl/core/neorv32_cpu_control.vhd:613:79  */
  assign n4168 = n3688 == 12'b101110011110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:613:79  */
  assign n4169 = n4166 | n4168;
  /* ../../rtl/core/neorv32_cpu_control.vhd:613:102  */
  assign n4171 = n3688 == 12'b101110011111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:613:102  */
  assign n4172 = n4169 | n4171;
  /* ../../rtl/core/neorv32_cpu_control.vhd:614:33  */
  assign n4174 = n3688 == 12'b001100100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:614:33  */
  assign n4175 = n4172 | n4174;
  /* ../../rtl/core/neorv32_cpu_control.vhd:615:33  */
  assign n4177 = n3688 == 12'b001100100100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:615:33  */
  assign n4178 = n4175 | n4177;
  /* ../../rtl/core/neorv32_cpu_control.vhd:615:56  */
  assign n4180 = n3688 == 12'b001100100101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:615:56  */
  assign n4181 = n4178 | n4180;
  /* ../../rtl/core/neorv32_cpu_control.vhd:615:79  */
  assign n4183 = n3688 == 12'b001100100110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:615:79  */
  assign n4184 = n4181 | n4183;
  /* ../../rtl/core/neorv32_cpu_control.vhd:615:102  */
  assign n4186 = n3688 == 12'b001100100111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:615:102  */
  assign n4187 = n4184 | n4186;
  /* ../../rtl/core/neorv32_cpu_control.vhd:616:33  */
  assign n4189 = n3688 == 12'b001100101000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:616:33  */
  assign n4190 = n4187 | n4189;
  /* ../../rtl/core/neorv32_cpu_control.vhd:616:56  */
  assign n4192 = n3688 == 12'b001100101001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:616:56  */
  assign n4193 = n4190 | n4192;
  /* ../../rtl/core/neorv32_cpu_control.vhd:616:79  */
  assign n4195 = n3688 == 12'b001100101010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:616:79  */
  assign n4196 = n4193 | n4195;
  /* ../../rtl/core/neorv32_cpu_control.vhd:616:102  */
  assign n4198 = n3688 == 12'b001100101011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:616:102  */
  assign n4199 = n4196 | n4198;
  /* ../../rtl/core/neorv32_cpu_control.vhd:617:33  */
  assign n4201 = n3688 == 12'b001100101100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:617:33  */
  assign n4202 = n4199 | n4201;
  /* ../../rtl/core/neorv32_cpu_control.vhd:617:56  */
  assign n4204 = n3688 == 12'b001100101101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:617:56  */
  assign n4205 = n4202 | n4204;
  /* ../../rtl/core/neorv32_cpu_control.vhd:617:79  */
  assign n4207 = n3688 == 12'b001100101110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:617:79  */
  assign n4208 = n4205 | n4207;
  /* ../../rtl/core/neorv32_cpu_control.vhd:617:102  */
  assign n4210 = n3688 == 12'b001100101111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:617:102  */
  assign n4211 = n4208 | n4210;
  /* ../../rtl/core/neorv32_cpu_control.vhd:618:33  */
  assign n4213 = n3688 == 12'b001100110000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:618:33  */
  assign n4214 = n4211 | n4213;
  /* ../../rtl/core/neorv32_cpu_control.vhd:618:56  */
  assign n4216 = n3688 == 12'b001100110001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:618:56  */
  assign n4217 = n4214 | n4216;
  /* ../../rtl/core/neorv32_cpu_control.vhd:618:79  */
  assign n4219 = n3688 == 12'b001100110010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:618:79  */
  assign n4220 = n4217 | n4219;
  /* ../../rtl/core/neorv32_cpu_control.vhd:618:102  */
  assign n4222 = n3688 == 12'b001100110011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:618:102  */
  assign n4223 = n4220 | n4222;
  /* ../../rtl/core/neorv32_cpu_control.vhd:619:33  */
  assign n4225 = n3688 == 12'b001100110100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:619:33  */
  assign n4226 = n4223 | n4225;
  /* ../../rtl/core/neorv32_cpu_control.vhd:619:56  */
  assign n4228 = n3688 == 12'b001100110101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:619:56  */
  assign n4229 = n4226 | n4228;
  /* ../../rtl/core/neorv32_cpu_control.vhd:619:79  */
  assign n4231 = n3688 == 12'b001100110110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:619:79  */
  assign n4232 = n4229 | n4231;
  /* ../../rtl/core/neorv32_cpu_control.vhd:619:102  */
  assign n4234 = n3688 == 12'b001100110111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:619:102  */
  assign n4235 = n4232 | n4234;
  /* ../../rtl/core/neorv32_cpu_control.vhd:620:33  */
  assign n4237 = n3688 == 12'b001100111000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:620:33  */
  assign n4238 = n4235 | n4237;
  /* ../../rtl/core/neorv32_cpu_control.vhd:620:56  */
  assign n4240 = n3688 == 12'b001100111001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:620:56  */
  assign n4241 = n4238 | n4240;
  /* ../../rtl/core/neorv32_cpu_control.vhd:620:79  */
  assign n4243 = n3688 == 12'b001100111010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:620:79  */
  assign n4244 = n4241 | n4243;
  /* ../../rtl/core/neorv32_cpu_control.vhd:620:102  */
  assign n4246 = n3688 == 12'b001100111011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:620:102  */
  assign n4247 = n4244 | n4246;
  /* ../../rtl/core/neorv32_cpu_control.vhd:621:33  */
  assign n4249 = n3688 == 12'b001100111100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:621:33  */
  assign n4250 = n4247 | n4249;
  /* ../../rtl/core/neorv32_cpu_control.vhd:621:56  */
  assign n4252 = n3688 == 12'b001100111101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:621:56  */
  assign n4253 = n4250 | n4252;
  /* ../../rtl/core/neorv32_cpu_control.vhd:621:79  */
  assign n4255 = n3688 == 12'b001100111110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:621:79  */
  assign n4256 = n4253 | n4255;
  /* ../../rtl/core/neorv32_cpu_control.vhd:621:102  */
  assign n4258 = n3688 == 12'b001100111111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:621:102  */
  assign n4259 = n4256 | n4258;
  /* ../../rtl/core/neorv32_cpu_control.vhd:622:33  */
  assign n4261 = n3688 == 12'b011100100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:622:33  */
  assign n4262 = n4259 | n4261;
  /* ../../rtl/core/neorv32_cpu_control.vhd:623:33  */
  assign n4264 = n3688 == 12'b011100100100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:623:33  */
  assign n4265 = n4262 | n4264;
  /* ../../rtl/core/neorv32_cpu_control.vhd:623:56  */
  assign n4267 = n3688 == 12'b011100100101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:623:56  */
  assign n4268 = n4265 | n4267;
  /* ../../rtl/core/neorv32_cpu_control.vhd:623:79  */
  assign n4270 = n3688 == 12'b011100100110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:623:79  */
  assign n4271 = n4268 | n4270;
  /* ../../rtl/core/neorv32_cpu_control.vhd:623:102  */
  assign n4273 = n3688 == 12'b011100100111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:623:102  */
  assign n4274 = n4271 | n4273;
  /* ../../rtl/core/neorv32_cpu_control.vhd:624:33  */
  assign n4276 = n3688 == 12'b011100101000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:624:33  */
  assign n4277 = n4274 | n4276;
  /* ../../rtl/core/neorv32_cpu_control.vhd:624:56  */
  assign n4279 = n3688 == 12'b011100101001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:624:56  */
  assign n4280 = n4277 | n4279;
  /* ../../rtl/core/neorv32_cpu_control.vhd:624:79  */
  assign n4282 = n3688 == 12'b011100101010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:624:79  */
  assign n4283 = n4280 | n4282;
  /* ../../rtl/core/neorv32_cpu_control.vhd:624:102  */
  assign n4285 = n3688 == 12'b011100101011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:624:102  */
  assign n4286 = n4283 | n4285;
  /* ../../rtl/core/neorv32_cpu_control.vhd:625:33  */
  assign n4288 = n3688 == 12'b011100101100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:625:33  */
  assign n4289 = n4286 | n4288;
  /* ../../rtl/core/neorv32_cpu_control.vhd:625:56  */
  assign n4291 = n3688 == 12'b011100101101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:625:56  */
  assign n4292 = n4289 | n4291;
  /* ../../rtl/core/neorv32_cpu_control.vhd:625:79  */
  assign n4294 = n3688 == 12'b011100101110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:625:79  */
  assign n4295 = n4292 | n4294;
  /* ../../rtl/core/neorv32_cpu_control.vhd:625:102  */
  assign n4297 = n3688 == 12'b011100101111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:625:102  */
  assign n4298 = n4295 | n4297;
  /* ../../rtl/core/neorv32_cpu_control.vhd:626:33  */
  assign n4300 = n3688 == 12'b011100110000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:626:33  */
  assign n4301 = n4298 | n4300;
  /* ../../rtl/core/neorv32_cpu_control.vhd:626:56  */
  assign n4303 = n3688 == 12'b011100110001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:626:56  */
  assign n4304 = n4301 | n4303;
  /* ../../rtl/core/neorv32_cpu_control.vhd:626:79  */
  assign n4306 = n3688 == 12'b011100110010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:626:79  */
  assign n4307 = n4304 | n4306;
  /* ../../rtl/core/neorv32_cpu_control.vhd:626:102  */
  assign n4309 = n3688 == 12'b011100110011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:626:102  */
  assign n4310 = n4307 | n4309;
  /* ../../rtl/core/neorv32_cpu_control.vhd:627:33  */
  assign n4312 = n3688 == 12'b011100110100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:627:33  */
  assign n4313 = n4310 | n4312;
  /* ../../rtl/core/neorv32_cpu_control.vhd:627:56  */
  assign n4315 = n3688 == 12'b011100110101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:627:56  */
  assign n4316 = n4313 | n4315;
  /* ../../rtl/core/neorv32_cpu_control.vhd:627:79  */
  assign n4318 = n3688 == 12'b011100110110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:627:79  */
  assign n4319 = n4316 | n4318;
  /* ../../rtl/core/neorv32_cpu_control.vhd:627:102  */
  assign n4321 = n3688 == 12'b011100110111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:627:102  */
  assign n4322 = n4319 | n4321;
  /* ../../rtl/core/neorv32_cpu_control.vhd:628:33  */
  assign n4324 = n3688 == 12'b011100111000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:628:33  */
  assign n4325 = n4322 | n4324;
  /* ../../rtl/core/neorv32_cpu_control.vhd:628:56  */
  assign n4327 = n3688 == 12'b011100111001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:628:56  */
  assign n4328 = n4325 | n4327;
  /* ../../rtl/core/neorv32_cpu_control.vhd:628:79  */
  assign n4330 = n3688 == 12'b011100111010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:628:79  */
  assign n4331 = n4328 | n4330;
  /* ../../rtl/core/neorv32_cpu_control.vhd:628:102  */
  assign n4333 = n3688 == 12'b011100111011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:628:102  */
  assign n4334 = n4331 | n4333;
  /* ../../rtl/core/neorv32_cpu_control.vhd:629:33  */
  assign n4336 = n3688 == 12'b011100111100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:629:33  */
  assign n4337 = n4334 | n4336;
  /* ../../rtl/core/neorv32_cpu_control.vhd:629:56  */
  assign n4339 = n3688 == 12'b011100111101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:629:56  */
  assign n4340 = n4337 | n4339;
  /* ../../rtl/core/neorv32_cpu_control.vhd:629:79  */
  assign n4342 = n3688 == 12'b011100111110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:629:79  */
  assign n4343 = n4340 | n4342;
  /* ../../rtl/core/neorv32_cpu_control.vhd:629:102  */
  assign n4345 = n3688 == 12'b011100111111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:629:102  */
  assign n4346 = n4343 | n4345;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:7  */
  assign n4350 = n3688 == 12'b110000000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:25  */
  assign n4352 = n3688 == 12'b110000000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:25  */
  assign n4353 = n4350 | n4352;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:39  */
  assign n4355 = n3688 == 12'b110000000010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:39  */
  assign n4356 = n4353 | n4355;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:56  */
  assign n4358 = n3688 == 12'b101100000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:56  */
  assign n4359 = n4356 | n4358;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:72  */
  assign n4361 = n3688 == 12'b101100000010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:72  */
  assign n4362 = n4359 | n4361;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:89  */
  assign n4364 = n3688 == 12'b110010000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:634:89  */
  assign n4365 = n4362 | n4364;
  /* ../../rtl/core/neorv32_cpu_control.vhd:635:25  */
  assign n4367 = n3688 == 12'b110010000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:635:25  */
  assign n4368 = n4365 | n4367;
  /* ../../rtl/core/neorv32_cpu_control.vhd:635:39  */
  assign n4370 = n3688 == 12'b110010000010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:635:39  */
  assign n4371 = n4368 | n4370;
  /* ../../rtl/core/neorv32_cpu_control.vhd:635:56  */
  assign n4373 = n3688 == 12'b101110000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:635:56  */
  assign n4374 = n4371 | n4373;
  /* ../../rtl/core/neorv32_cpu_control.vhd:635:72  */
  assign n4376 = n3688 == 12'b101110000010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:635:72  */
  assign n4377 = n4374 | n4376;
  /* ../../rtl/core/neorv32_cpu_control.vhd:639:7  */
  assign n4381 = n3688 == 12'b001100100001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:639:28  */
  assign n4383 = n3688 == 12'b001100100010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:639:28  */
  assign n4384 = n4381 | n4383;
  /* ../../rtl/core/neorv32_cpu_control.vhd:639:48  */
  assign n4386 = n3688 == 12'b011100100001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:639:48  */
  assign n4387 = n4384 | n4386;
  /* ../../rtl/core/neorv32_cpu_control.vhd:639:67  */
  assign n4389 = n3688 == 12'b011100100010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:639:67  */
  assign n4390 = n4387 | n4389;
  /* ../../rtl/core/neorv32_cpu_control.vhd:643:7  */
  assign n4394 = n3688 == 12'b011110110000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:643:23  */
  assign n4396 = n3688 == 12'b011110110001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:643:23  */
  assign n4397 = n4394 | n4396;
  /* ../../rtl/core/neorv32_cpu_control.vhd:643:35  */
  assign n4399 = n3688 == 12'b011110110010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:643:35  */
  assign n4400 = n4397 | n4399;
  /* ../../rtl/core/neorv32_cpu_control.vhd:647:7  */
  assign n4404 = n3688 == 12'b011110100000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:647:26  */
  assign n4406 = n3688 == 12'b011110100001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:647:26  */
  assign n4407 = n4404 | n4406;
  /* ../../rtl/core/neorv32_cpu_control.vhd:647:41  */
  assign n4409 = n3688 == 12'b011110100010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:647:41  */
  assign n4410 = n4407 | n4409;
  /* ../../rtl/core/neorv32_cpu_control.vhd:647:56  */
  assign n4412 = n3688 == 12'b011110100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:647:56  */
  assign n4413 = n4410 | n4412;
  /* ../../rtl/core/neorv32_cpu_control.vhd:647:71  */
  assign n4415 = n3688 == 12'b011110100100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:647:71  */
  assign n4416 = n4413 | n4415;
  assign n4418 = {n4416, n4400, n4390, n4377, n4346, n3823, n3762, n3752, n3698};
  /* ../../rtl/core/neorv32_cpu_control.vhd:559:5  */
  always @*
    case (n4418)
      9'b100000000: n4419 = 1'b0;
      9'b010000000: n4419 = 1'b0;
      9'b001000000: n4419 = 1'b0;
      9'b000100000: n4419 = 1'b1;
      9'b000010000: n4419 = 1'b0;
      9'b000001000: n4419 = 1'b0;
      9'b000000100: n4419 = 1'b0;
      9'b000000010: n4419 = 1'b1;
      9'b000000001: n4419 = 1'b0;
      default: n4419 = 1'b0;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:659:22  */
  assign n4420 = ctrl[176:175]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:659:37  */
  assign n4422 = n4420 == 2'b11;
  /* ../../rtl/core/neorv32_cpu_control.vhd:660:17  */
  assign n4423 = exec[18:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:660:64  */
  assign n4425 = n4423 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:661:17  */
  assign n4426 = exec[18:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:661:64  */
  assign n4428 = n4426 == 3'b101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:660:83  */
  assign n4429 = n4425 | n4428;
  /* ../../rtl/core/neorv32_cpu_control.vhd:662:17  */
  assign n4430 = exec[23:19]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:662:58  */
  assign n4432 = n4430 != 5'b00000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:661:83  */
  assign n4433 = n4429 | n4432;
  /* ../../rtl/core/neorv32_cpu_control.vhd:659:45  */
  assign n4434 = n4433 & n4422;
  /* ../../rtl/core/neorv32_cpu_control.vhd:659:5  */
  assign n4437 = n4434 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:678:25  */
  assign n4441 = ctrl[174:173]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:678:38  */
  assign n4443 = n4441 != 2'b00;
  /* ../../rtl/core/neorv32_cpu_control.vhd:678:56  */
  assign n4444 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:678:66  */
  assign n4445 = ~n4444;
  /* ../../rtl/core/neorv32_cpu_control.vhd:678:47  */
  assign n4446 = n4445 & n4443;
  /* ../../rtl/core/neorv32_cpu_control.vhd:678:5  */
  assign n4449 = n4446 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:691:17  */
  assign n4452 = exec[10:4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:694:7  */
  assign n4454 = n4452 == 7'b0110111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:694:25  */
  assign n4456 = n4452 == 7'b0010111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:694:25  */
  assign n4457 = n4454 | n4456;
  /* ../../rtl/core/neorv32_cpu_control.vhd:694:42  */
  assign n4459 = n4452 == 7'b1101111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:694:42  */
  assign n4460 = n4457 | n4459;
  /* ../../rtl/core/neorv32_cpu_control.vhd:699:20  */
  assign n4461 = exec[18:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:699:67  */
  assign n4463 = n4461 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:699:9  */
  assign n4466 = n4463 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:698:7  */
  assign n4468 = n4452 == 7'b1100111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:705:20  */
  assign n4469 = exec[18:17]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:705:69  */
  assign n4471 = n4469 != 2'b01;
  /* ../../rtl/core/neorv32_cpu_control.vhd:705:78  */
  assign n4473 = n4471 | 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:705:9  */
  assign n4476 = n4473 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:704:7  */
  assign n4478 = n4452 == 7'b1100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:711:21  */
  assign n4479 = exec[18:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:712:11  */
  assign n4481 = n4479 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:712:28  */
  assign n4483 = n4479 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:712:28  */
  assign n4484 = n4481 | n4483;
  /* ../../rtl/core/neorv32_cpu_control.vhd:712:42  */
  assign n4486 = n4479 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:712:42  */
  assign n4487 = n4484 | n4486;
  /* ../../rtl/core/neorv32_cpu_control.vhd:712:56  */
  assign n4489 = n4479 == 3'b100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:712:56  */
  assign n4490 = n4487 | n4489;
  /* ../../rtl/core/neorv32_cpu_control.vhd:712:71  */
  assign n4492 = n4479 == 3'b101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:712:71  */
  assign n4493 = n4490 | n4492;
  /* ../../rtl/core/neorv32_cpu_control.vhd:711:9  */
  always @*
    case (n4493)
      1'b1: n4496 = 1'b0;
      default: n4496 = 1'b1;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:710:7  */
  assign n4498 = n4452 == 7'b0000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:718:21  */
  assign n4499 = exec[18:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:719:11  */
  assign n4501 = n4499 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:719:28  */
  assign n4503 = n4499 == 3'b001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:719:28  */
  assign n4504 = n4501 | n4503;
  /* ../../rtl/core/neorv32_cpu_control.vhd:719:42  */
  assign n4506 = n4499 == 3'b010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:719:42  */
  assign n4507 = n4504 | n4506;
  /* ../../rtl/core/neorv32_cpu_control.vhd:718:9  */
  always @*
    case (n4507)
      1'b1: n4510 = 1'b0;
      default: n4510 = 1'b1;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:717:7  */
  assign n4512 = n4452 == 7'b0100011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:724:7  */
  assign n4552 = n4452 == 7'b0101111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:7  */
  assign n4554 = n4452 == 7'b0110011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:25  */
  assign n4556 = n4452 == 7'b0010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:25  */
  assign n4557 = n4554 | n4556;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:41  */
  assign n4559 = n4452 == 7'b1010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:41  */
  assign n4560 = n4557 | n4559;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:56  */
  assign n4562 = n4452 == 7'b0111011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:56  */
  assign n4563 = n4560 | n4562;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:72  */
  assign n4565 = n4452 == 7'b0011011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:72  */
  assign n4566 = n4563 | n4565;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:89  */
  assign n4568 = n4452 == 7'b0001011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:89  */
  assign n4569 = n4566 | n4568;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:106  */
  assign n4571 = n4452 == 7'b0101011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:734:106  */
  assign n4572 = n4569 | n4571;
  /* ../../rtl/core/neorv32_cpu_control.vhd:739:20  */
  assign n4573 = exec[18:17]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:739:69  */
  assign n4575 = n4573 == 2'b00;
  /* ../../rtl/core/neorv32_cpu_control.vhd:739:9  */
  assign n4578 = n4575 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:738:7  */
  assign n4580 = n4452 == 7'b0001111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:745:20  */
  assign n4581 = exec[18:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:745:67  */
  assign n4583 = n4581 == 3'b000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:746:22  */
  assign n4584 = exec[23:19]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:746:63  */
  assign n4586 = n4584 == 5'b00000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:746:86  */
  assign n4587 = exec[15:11]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:746:125  */
  assign n4589 = n4587 == 5'b00000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:746:74  */
  assign n4590 = n4589 & n4586;
  /* ../../rtl/core/neorv32_cpu_control.vhd:747:25  */
  assign n4591 = exec[35:24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:748:15  */
  assign n4593 = n4591 == 12'b000000000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:749:15  */
  assign n4595 = n4591 == 12'b000000000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:750:64  */
  assign n4596 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:750:56  */
  assign n4597 = ~n4596;
  /* ../../rtl/core/neorv32_cpu_control.vhd:750:89  */
  assign n4598 = debug_ctrl[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:750:75  */
  assign n4599 = n4597 | n4598;
  /* ../../rtl/core/neorv32_cpu_control.vhd:750:15  */
  assign n4601 = n4591 == 12'b001100000010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:751:70  */
  assign n4602 = debug_ctrl[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:751:55  */
  assign n4603 = ~n4602;
  /* ../../rtl/core/neorv32_cpu_control.vhd:751:15  */
  assign n4605 = n4591 == 12'b011110110010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:752:64  */
  assign n4606 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:752:56  */
  assign n4607 = ~n4606;
  /* ../../rtl/core/neorv32_cpu_control.vhd:752:83  */
  assign n4608 = csr[5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:752:75  */
  assign n4609 = n4607 & n4608;
  /* ../../rtl/core/neorv32_cpu_control.vhd:752:15  */
  assign n4611 = n4591 == 12'b000100000101;
  assign n4612 = {n4611, n4605, n4601, n4595, n4593};
  /* ../../rtl/core/neorv32_cpu_control.vhd:747:13  */
  always @*
    case (n4612)
      5'b10000: n4616 = n4609;
      5'b01000: n4616 = n4603;
      5'b00100: n4616 = n4599;
      5'b00010: n4616 = 1'b0;
      5'b00001: n4616 = 1'b0;
      default: n4616 = 1'b1;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:746:11  */
  assign n4618 = n4590 ? n4616 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:756:23  */
  assign n4619 = exec[18:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:756:70  */
  assign n4621 = n4619 == 3'b100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:758:26  */
  assign n4624 = csr_valid == 3'b111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:758:9  */
  assign n4627 = n4624 ? 1'b0 : 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:756:9  */
  assign n4629 = n4621 ? 1'b1 : n4627;
  /* ../../rtl/core/neorv32_cpu_control.vhd:745:9  */
  assign n4630 = n4583 ? n4618 : n4629;
  /* ../../rtl/core/neorv32_cpu_control.vhd:744:7  */
  assign n4632 = n4452 == 7'b1110011;
  assign n4633 = {n4632, n4580, n4572, n4552, n4512, n4498, n4478, n4468, n4460};
  /* ../../rtl/core/neorv32_cpu_control.vhd:691:5  */
  always @*
    case (n4633)
      9'b100000000: n4638 = n4630;
      9'b010000000: n4638 = n4578;
      9'b001000000: n4638 = 1'b0;
      9'b000100000: n4638 = 1'b1;
      9'b000010000: n4638 = n4510;
      9'b000001000: n4638 = n4496;
      9'b000000100: n4638 = n4476;
      9'b000000010: n4638 = n4466;
      9'b000000001: n4638 = 1'b0;
      default: n4638 = 1'b1;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:774:16  */
  assign n4642 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_control.vhd:777:16  */
  assign n4644 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:777:22  */
  assign n4646 = n4644 == 4'b0101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:778:64  */
  assign n4648 = monitor_cnt + 10'b0000000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:777:7  */
  assign n4650 = n4646 ? n4648 : 10'b0000000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:788:36  */
  assign n4656 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:788:42  */
  assign n4658 = n4656 == 4'b0100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:788:64  */
  assign n4659 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:788:70  */
  assign n4661 = n4659 == 4'b0101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:788:55  */
  assign n4662 = n4658 | n4661;
  /* ../../rtl/core/neorv32_cpu_control.vhd:789:42  */
  assign n4663 = monitor_cnt[9]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:789:68  */
  assign n4664 = n4663 | illegal_cmd;
  /* ../../rtl/core/neorv32_cpu_control.vhd:788:85  */
  assign n4665 = n4664 & n4662;
  /* ../../rtl/core/neorv32_cpu_control.vhd:788:24  */
  assign n4666 = n4665 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:800:16  */
  assign n4669 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_control.vhd:806:27  */
  assign n4675 = {1'b0, irq_fast_i};
  /* ../../rtl/core/neorv32_cpu_control.vhd:806:40  */
  assign n4676 = {n4675, irq_machine_i};
  /* ../../rtl/core/neorv32_cpu_control.vhd:808:49  */
  assign n4677 = debug_ctrl[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:808:93  */
  assign n4678 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:808:118  */
  assign n4679 = trap[52]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:808:102  */
  assign n4680 = n4678 & n4679;
  /* ../../rtl/core/neorv32_cpu_control.vhd:808:84  */
  assign n4681 = n4677 | n4680;
  /* ../../rtl/core/neorv32_cpu_control.vhd:809:51  */
  assign n4682 = trap[13]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:809:75  */
  assign n4683 = csr[6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:809:67  */
  assign n4684 = n4682 & n4683;
  /* ../../rtl/core/neorv32_cpu_control.vhd:809:93  */
  assign n4685 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:809:118  */
  assign n4686 = trap[33]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:809:102  */
  assign n4687 = n4685 & n4686;
  /* ../../rtl/core/neorv32_cpu_control.vhd:809:84  */
  assign n4688 = n4684 | n4687;
  /* ../../rtl/core/neorv32_cpu_control.vhd:810:51  */
  assign n4689 = trap[15]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:810:75  */
  assign n4690 = csr[7]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:810:67  */
  assign n4691 = n4689 & n4690;
  /* ../../rtl/core/neorv32_cpu_control.vhd:810:93  */
  assign n4692 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:810:118  */
  assign n4693 = trap[35]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:810:102  */
  assign n4694 = n4692 & n4693;
  /* ../../rtl/core/neorv32_cpu_control.vhd:810:84  */
  assign n4695 = n4691 | n4694;
  /* ../../rtl/core/neorv32_cpu_control.vhd:811:51  */
  assign n4696 = trap[14]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:811:75  */
  assign n4697 = csr[8]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:811:67  */
  assign n4698 = n4696 & n4697;
  /* ../../rtl/core/neorv32_cpu_control.vhd:811:93  */
  assign n4699 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:811:118  */
  assign n4700 = trap[34]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:811:102  */
  assign n4701 = n4699 & n4700;
  /* ../../rtl/core/neorv32_cpu_control.vhd:811:84  */
  assign n4702 = n4698 | n4701;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4703 = trap[16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4704 = csr[9]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4705 = n4703 & n4704;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4706 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4707 = trap[36]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4708 = n4706 & n4707;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4709 = n4705 | n4708;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4710 = trap[17]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4711 = csr[10]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4712 = n4710 & n4711;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4713 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4714 = trap[37]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4715 = n4713 & n4714;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4716 = n4712 | n4715;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4717 = trap[18]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4718 = csr[11]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4719 = n4717 & n4718;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4720 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4721 = trap[38]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4722 = n4720 & n4721;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4723 = n4719 | n4722;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4724 = trap[19]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4725 = csr[12]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4726 = n4724 & n4725;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4727 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4728 = trap[39]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4729 = n4727 & n4728;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4730 = n4726 | n4729;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4731 = trap[20]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4732 = csr[13]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4733 = n4731 & n4732;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4734 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4735 = trap[40]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4736 = n4734 & n4735;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4737 = n4733 | n4736;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4738 = trap[21]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4739 = csr[14]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4740 = n4738 & n4739;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4741 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4742 = trap[41]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4743 = n4741 & n4742;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4744 = n4740 | n4743;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4745 = trap[22]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4746 = csr[15]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4747 = n4745 & n4746;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4748 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4749 = trap[42]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4750 = n4748 & n4749;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4751 = n4747 | n4750;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4752 = trap[23]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4753 = csr[16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4754 = n4752 & n4753;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4755 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4756 = trap[43]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4757 = n4755 & n4756;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4758 = n4754 | n4757;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4759 = trap[24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4760 = csr[17]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4761 = n4759 & n4760;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4762 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4763 = trap[44]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4764 = n4762 & n4763;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4765 = n4761 | n4764;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4766 = trap[25]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4767 = csr[18]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4768 = n4766 & n4767;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4769 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4770 = trap[45]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4771 = n4769 & n4770;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4772 = n4768 | n4771;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4773 = trap[26]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4774 = csr[19]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4775 = n4773 & n4774;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4776 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4777 = trap[46]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4778 = n4776 & n4777;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4779 = n4775 | n4778;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4780 = trap[27]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4781 = csr[20]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4782 = n4780 & n4781;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4783 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4784 = trap[47]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4785 = n4783 & n4784;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4786 = n4782 | n4785;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4787 = trap[28]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4788 = csr[21]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4789 = n4787 & n4788;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4790 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4791 = trap[48]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4792 = n4790 & n4791;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4793 = n4789 | n4792;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4794 = trap[29]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4795 = csr[22]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4796 = n4794 & n4795;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4797 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4798 = trap[49]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4799 = n4797 & n4798;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4800 = n4796 | n4799;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4801 = trap[30]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4802 = csr[23]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4803 = n4801 & n4802;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4804 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4805 = trap[50]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4806 = n4804 & n4805;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4807 = n4803 | n4806;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:54  */
  assign n4808 = trap[31]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:87  */
  assign n4809 = csr[24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:71  */
  assign n4810 = n4808 & n4809;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:101  */
  assign n4811 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:126  */
  assign n4812 = trap[51]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:110  */
  assign n4813 = n4811 & n4812;
  /* ../../rtl/core/neorv32_cpu_control.vhd:813:92  */
  assign n4814 = n4810 | n4813;
  /* ../../rtl/core/neorv32_cpu_control.vhd:816:51  */
  assign n4815 = trap[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:816:75  */
  assign n4816 = trap[97]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:816:67  */
  assign n4817 = n4815 | n4816;
  /* ../../rtl/core/neorv32_cpu_control.vhd:816:107  */
  assign n4818 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:816:98  */
  assign n4819 = ~n4818;
  /* ../../rtl/core/neorv32_cpu_control.vhd:816:93  */
  assign n4820 = n4817 & n4819;
  /* ../../rtl/core/neorv32_cpu_control.vhd:817:51  */
  assign n4821 = trap[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:817:75  */
  assign n4822 = trap[99]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:817:67  */
  assign n4823 = n4821 | n4822;
  /* ../../rtl/core/neorv32_cpu_control.vhd:817:107  */
  assign n4824 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:817:98  */
  assign n4825 = ~n4824;
  /* ../../rtl/core/neorv32_cpu_control.vhd:817:93  */
  assign n4826 = n4823 & n4825;
  /* ../../rtl/core/neorv32_cpu_control.vhd:818:51  */
  assign n4827 = trap[2]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:818:75  */
  assign n4828 = trap[98]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:818:67  */
  assign n4829 = n4827 | n4828;
  /* ../../rtl/core/neorv32_cpu_control.vhd:818:107  */
  assign n4830 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:818:98  */
  assign n4831 = ~n4830;
  /* ../../rtl/core/neorv32_cpu_control.vhd:818:93  */
  assign n4832 = n4829 & n4831;
  /* ../../rtl/core/neorv32_cpu_control.vhd:819:51  */
  assign n4833 = trap[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:819:75  */
  assign n4834 = trap[100]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:819:67  */
  assign n4835 = n4833 | n4834;
  /* ../../rtl/core/neorv32_cpu_control.vhd:819:107  */
  assign n4836 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:819:98  */
  assign n4837 = ~n4836;
  /* ../../rtl/core/neorv32_cpu_control.vhd:819:93  */
  assign n4838 = n4835 & n4837;
  /* ../../rtl/core/neorv32_cpu_control.vhd:820:51  */
  assign n4839 = trap[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:820:67  */
  assign n4840 = n4839 | ebreak_trig;
  /* ../../rtl/core/neorv32_cpu_control.vhd:820:107  */
  assign n4841 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:820:98  */
  assign n4842 = ~n4841;
  /* ../../rtl/core/neorv32_cpu_control.vhd:820:93  */
  assign n4843 = n4840 & n4842;
  /* ../../rtl/core/neorv32_cpu_control.vhd:821:51  */
  assign n4844 = trap[5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:821:79  */
  assign n4845 = lsu_err_i[2]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:821:67  */
  assign n4846 = n4844 | n4845;
  /* ../../rtl/core/neorv32_cpu_control.vhd:821:107  */
  assign n4847 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:821:98  */
  assign n4848 = ~n4847;
  /* ../../rtl/core/neorv32_cpu_control.vhd:821:93  */
  assign n4849 = n4846 & n4848;
  /* ../../rtl/core/neorv32_cpu_control.vhd:822:51  */
  assign n4850 = trap[6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:822:79  */
  assign n4851 = lsu_err_i[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:822:67  */
  assign n4852 = n4850 | n4851;
  /* ../../rtl/core/neorv32_cpu_control.vhd:822:107  */
  assign n4853 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:822:98  */
  assign n4854 = ~n4853;
  /* ../../rtl/core/neorv32_cpu_control.vhd:822:93  */
  assign n4855 = n4852 & n4854;
  /* ../../rtl/core/neorv32_cpu_control.vhd:823:51  */
  assign n4856 = trap[7]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:823:79  */
  assign n4857 = lsu_err_i[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:823:67  */
  assign n4858 = n4856 | n4857;
  /* ../../rtl/core/neorv32_cpu_control.vhd:823:107  */
  assign n4859 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:823:98  */
  assign n4860 = ~n4859;
  /* ../../rtl/core/neorv32_cpu_control.vhd:823:93  */
  assign n4861 = n4858 & n4860;
  /* ../../rtl/core/neorv32_cpu_control.vhd:824:51  */
  assign n4862 = trap[8]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:824:79  */
  assign n4863 = lsu_err_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:824:67  */
  assign n4864 = n4862 | n4863;
  /* ../../rtl/core/neorv32_cpu_control.vhd:824:107  */
  assign n4865 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:824:98  */
  assign n4866 = ~n4865;
  /* ../../rtl/core/neorv32_cpu_control.vhd:824:93  */
  assign n4867 = n4864 & n4866;
  /* ../../rtl/core/neorv32_cpu_control.vhd:825:51  */
  assign n4868 = trap[9]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:825:81  */
  assign n4869 = debug_ctrl[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:825:67  */
  assign n4870 = n4868 | n4869;
  /* ../../rtl/core/neorv32_cpu_control.vhd:825:107  */
  assign n4871 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:825:98  */
  assign n4872 = ~n4871;
  /* ../../rtl/core/neorv32_cpu_control.vhd:825:93  */
  assign n4873 = n4870 & n4872;
  /* ../../rtl/core/neorv32_cpu_control.vhd:826:51  */
  assign n4874 = trap[10]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:826:81  */
  assign n4875 = debug_ctrl[2]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:826:67  */
  assign n4876 = n4874 | n4875;
  /* ../../rtl/core/neorv32_cpu_control.vhd:826:107  */
  assign n4877 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:826:98  */
  assign n4878 = ~n4877;
  /* ../../rtl/core/neorv32_cpu_control.vhd:826:93  */
  assign n4879 = n4876 & n4878;
  /* ../../rtl/core/neorv32_cpu_control.vhd:827:51  */
  assign n4880 = trap[11]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:827:81  */
  assign n4881 = debug_ctrl[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:827:67  */
  assign n4882 = n4880 | n4881;
  /* ../../rtl/core/neorv32_cpu_control.vhd:827:107  */
  assign n4883 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:827:98  */
  assign n4884 = ~n4883;
  /* ../../rtl/core/neorv32_cpu_control.vhd:827:93  */
  assign n4885 = n4882 & n4884;
  assign n4886 = {n4885, n4879, n4873, n4867, n4861, n4855, n4849, n4843, n4838, n4832, n4826, n4820};
  assign n4887 = {n4681, n4814, n4807, n4800, n4793, n4786, n4779, n4772, n4765, n4758, n4751, n4744, n4737, n4730, n4723, n4716, n4709, n4695, n4702, n4688, n4676};
  assign n4892 = {20'b00000000000000000000, 20'b00000000000000000000};
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:24  */
  assign n4896 = trap[101]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:44  */
  assign n4897 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:31  */
  assign n4898 = n4896 & n4897;
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:68  */
  assign n4899 = csr[191]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:60  */
  assign n4900 = ~n4899;
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:55  */
  assign n4901 = n4898 & n4900;
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:102  */
  assign n4902 = debug_ctrl[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:87  */
  assign n4903 = ~n4902;
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:82  */
  assign n4904 = n4901 & n4903;
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:24  */
  assign n4905 = trap[101]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:44  */
  assign n4906 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:36  */
  assign n4907 = ~n4906;
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:31  */
  assign n4908 = n4905 & n4907;
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:68  */
  assign n4909 = csr[192]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:60  */
  assign n4910 = ~n4909;
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:55  */
  assign n4911 = n4908 & n4910;
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:102  */
  assign n4912 = debug_ctrl[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:87  */
  assign n4913 = ~n4912;
  /* ../../rtl/core/neorv32_cpu_control.vhd:833:82  */
  assign n4914 = n4911 & n4913;
  /* ../../rtl/core/neorv32_cpu_control.vhd:832:108  */
  assign n4915 = n4904 | n4914;
  /* ../../rtl/core/neorv32_cpu_control.vhd:840:16  */
  assign n4917 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_control.vhd:843:16  */
  assign n4920 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:845:19  */
  assign n4922 = trap[12]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4930 = trap[54]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4932 = 1'b0 | n4930;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4934 = trap[53]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4935 = n4932 | n4934;
  /* ../../rtl/core/neorv32_cpu_control.vhd:845:35  */
  assign n4936 = n4922 | n4935;
  assign n4938 = trap[94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:845:7  */
  assign n4939 = n4936 ? 1'b1 : n4938;
  /* ../../rtl/core/neorv32_cpu_control.vhd:843:7  */
  assign n4940 = n4920 ? 1'b0 : n4939;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4952 = trap[11]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4954 = 1'b0 | n4952;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4956 = trap[10]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4957 = n4954 | n4956;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4958 = trap[9]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4959 = n4957 | n4958;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4960 = trap[8]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4961 = n4959 | n4960;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4962 = trap[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4963 = n4961 | n4962;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4964 = trap[6]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4965 = n4963 | n4964;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4966 = trap[5]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4967 = n4965 | n4966;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4968 = trap[4]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4969 = n4967 | n4968;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4970 = trap[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4971 = n4969 | n4970;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4972 = trap[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4973 = n4971 | n4972;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4974 = trap[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4975 = n4973 | n4974;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4976 = trap[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4977 = n4975 | n4976;
  /* ../../rtl/core/neorv32_cpu_control.vhd:856:12  */
  assign n4979 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:856:18  */
  assign n4981 = n4979 == 4'b0100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:856:40  */
  assign n4982 = exec[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:856:46  */
  assign n4984 = n4982 == 4'b1010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:856:31  */
  assign n4985 = n4981 | n4984;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4993 = trap[51]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4995 = 1'b0 | n4993;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4997 = trap[50]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n4998 = n4995 | n4997;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n4999 = trap[49]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5000 = n4998 | n4999;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5001 = trap[48]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5002 = n5000 | n5001;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5003 = trap[47]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5004 = n5002 | n5003;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5005 = trap[46]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5006 = n5004 | n5005;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5007 = trap[45]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5008 = n5006 | n5007;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5009 = trap[44]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5010 = n5008 | n5009;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5011 = trap[43]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5012 = n5010 | n5011;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5013 = trap[42]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5014 = n5012 | n5013;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5015 = trap[41]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5016 = n5014 | n5015;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5017 = trap[40]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5018 = n5016 | n5017;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5019 = trap[39]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5020 = n5018 | n5019;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5021 = trap[38]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5022 = n5020 | n5021;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5023 = trap[37]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5024 = n5022 | n5023;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5025 = trap[36]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5026 = n5024 | n5025;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5027 = trap[35]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5028 = n5026 | n5027;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5029 = trap[34]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5030 = n5028 | n5029;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5031 = trap[33]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5032 = n5030 | n5031;
  /* ../../rtl/core/neorv32_cpu_control.vhd:856:58  */
  assign n5033 = n5032 & n4985;
  /* ../../rtl/core/neorv32_cpu_control.vhd:858:11  */
  assign n5034 = csr[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:858:38  */
  assign n5035 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:858:48  */
  assign n5036 = ~n5035;
  /* ../../rtl/core/neorv32_cpu_control.vhd:858:30  */
  assign n5037 = n5034 | n5036;
  /* ../../rtl/core/neorv32_cpu_control.vhd:857:75  */
  assign n5038 = n5037 & n5033;
  /* ../../rtl/core/neorv32_cpu_control.vhd:859:17  */
  assign n5039 = debug_ctrl[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:859:21  */
  assign n5040 = ~n5039;
  /* ../../rtl/core/neorv32_cpu_control.vhd:858:66  */
  assign n5041 = n5040 & n5038;
  /* ../../rtl/core/neorv32_cpu_control.vhd:859:37  */
  assign n5042 = csr[193]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:859:47  */
  assign n5043 = ~n5042;
  /* ../../rtl/core/neorv32_cpu_control.vhd:859:28  */
  assign n5044 = n5043 & n5041;
  /* ../../rtl/core/neorv32_cpu_control.vhd:855:27  */
  assign n5045 = n5044 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:862:35  */
  assign n5047 = trap[52]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:869:38  */
  assign n5049 = trap[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:869:20  */
  assign n5050 = n5049 ? 7'b0000001 : n5053;
  /* ../../rtl/core/neorv32_cpu_control.vhd:870:38  */
  assign n5052 = trap[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:869:61  */
  assign n5053 = n5052 ? 7'b0000010 : n5056;
  /* ../../rtl/core/neorv32_cpu_control.vhd:871:38  */
  assign n5055 = trap[2]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:870:61  */
  assign n5056 = n5055 ? 7'b0000000 : n5058;
  /* ../../rtl/core/neorv32_cpu_control.vhd:872:38  */
  assign n5057 = trap[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:871:61  */
  assign n5058 = n5057 ? trap_env : n5061;
  /* ../../rtl/core/neorv32_cpu_control.vhd:873:38  */
  assign n5060 = trap[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:872:61  */
  assign n5061 = n5060 ? 7'b0000011 : n5064;
  /* ../../rtl/core/neorv32_cpu_control.vhd:874:38  */
  assign n5063 = trap[5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:873:61  */
  assign n5064 = n5063 ? 7'b0000110 : n5067;
  /* ../../rtl/core/neorv32_cpu_control.vhd:875:38  */
  assign n5066 = trap[6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:874:61  */
  assign n5067 = n5066 ? 7'b0000100 : n5070;
  /* ../../rtl/core/neorv32_cpu_control.vhd:876:38  */
  assign n5069 = trap[7]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:875:61  */
  assign n5070 = n5069 ? 7'b0000111 : n5073;
  /* ../../rtl/core/neorv32_cpu_control.vhd:877:38  */
  assign n5072 = trap[8]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:876:61  */
  assign n5073 = n5072 ? 7'b0000101 : n5076;
  /* ../../rtl/core/neorv32_cpu_control.vhd:879:38  */
  assign n5075 = trap[52]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:877:61  */
  assign n5076 = n5075 ? 7'b1100011 : n5079;
  /* ../../rtl/core/neorv32_cpu_control.vhd:880:38  */
  assign n5078 = trap[10]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:879:61  */
  assign n5079 = n5078 ? 7'b1100010 : n5082;
  /* ../../rtl/core/neorv32_cpu_control.vhd:881:38  */
  assign n5081 = trap[9]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:880:61  */
  assign n5082 = n5081 ? 7'b0100001 : n5085;
  /* ../../rtl/core/neorv32_cpu_control.vhd:882:38  */
  assign n5084 = trap[11]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:881:61  */
  assign n5085 = n5084 ? 7'b1100100 : n5088;
  /* ../../rtl/core/neorv32_cpu_control.vhd:884:38  */
  assign n5087 = trap[36]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:882:61  */
  assign n5088 = n5087 ? 7'b1010000 : n5091;
  /* ../../rtl/core/neorv32_cpu_control.vhd:885:38  */
  assign n5090 = trap[37]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:884:61  */
  assign n5091 = n5090 ? 7'b1010001 : n5094;
  /* ../../rtl/core/neorv32_cpu_control.vhd:886:38  */
  assign n5093 = trap[38]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:885:61  */
  assign n5094 = n5093 ? 7'b1010010 : n5097;
  /* ../../rtl/core/neorv32_cpu_control.vhd:887:38  */
  assign n5096 = trap[39]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:886:61  */
  assign n5097 = n5096 ? 7'b1010011 : n5100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:888:38  */
  assign n5099 = trap[40]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:887:61  */
  assign n5100 = n5099 ? 7'b1010100 : n5103;
  /* ../../rtl/core/neorv32_cpu_control.vhd:889:38  */
  assign n5102 = trap[41]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:888:61  */
  assign n5103 = n5102 ? 7'b1010101 : n5106;
  /* ../../rtl/core/neorv32_cpu_control.vhd:890:38  */
  assign n5105 = trap[42]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:889:61  */
  assign n5106 = n5105 ? 7'b1010110 : n5109;
  /* ../../rtl/core/neorv32_cpu_control.vhd:891:38  */
  assign n5108 = trap[43]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:890:61  */
  assign n5109 = n5108 ? 7'b1010111 : n5112;
  /* ../../rtl/core/neorv32_cpu_control.vhd:892:38  */
  assign n5111 = trap[44]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:891:61  */
  assign n5112 = n5111 ? 7'b1011000 : n5115;
  /* ../../rtl/core/neorv32_cpu_control.vhd:893:38  */
  assign n5114 = trap[45]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:892:61  */
  assign n5115 = n5114 ? 7'b1011001 : n5118;
  /* ../../rtl/core/neorv32_cpu_control.vhd:894:38  */
  assign n5117 = trap[46]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:893:61  */
  assign n5118 = n5117 ? 7'b1011010 : n5121;
  /* ../../rtl/core/neorv32_cpu_control.vhd:895:38  */
  assign n5120 = trap[47]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:894:61  */
  assign n5121 = n5120 ? 7'b1011011 : n5124;
  /* ../../rtl/core/neorv32_cpu_control.vhd:896:38  */
  assign n5123 = trap[48]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:895:61  */
  assign n5124 = n5123 ? 7'b1011100 : n5127;
  /* ../../rtl/core/neorv32_cpu_control.vhd:897:38  */
  assign n5126 = trap[49]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:896:61  */
  assign n5127 = n5126 ? 7'b1011101 : n5130;
  /* ../../rtl/core/neorv32_cpu_control.vhd:898:38  */
  assign n5129 = trap[50]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:897:61  */
  assign n5130 = n5129 ? 7'b1011110 : n5133;
  /* ../../rtl/core/neorv32_cpu_control.vhd:899:38  */
  assign n5132 = trap[51]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:898:61  */
  assign n5133 = n5132 ? 7'b1011111 : n5136;
  /* ../../rtl/core/neorv32_cpu_control.vhd:901:38  */
  assign n5135 = trap[35]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:899:61  */
  assign n5136 = n5135 ? 7'b1001011 : n5139;
  /* ../../rtl/core/neorv32_cpu_control.vhd:902:38  */
  assign n5138 = trap[33]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:901:61  */
  assign n5139 = n5138 ? 7'b1000011 : 7'b1000111;
  /* ../../rtl/core/neorv32_cpu_control.vhd:906:44  */
  assign n5141 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:906:38  */
  assign n5143 = {5'b00010, n5141};
  /* ../../rtl/core/neorv32_cpu_control.vhd:906:60  */
  assign n5144 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:906:54  */
  assign n5145 = {n5143, n5144};
  /* ../../rtl/core/neorv32_cpu_control.vhd:909:19  */
  assign n5146 = exec[116:85]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:909:39  */
  assign n5147 = trap[61]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:909:23  */
  assign n5148 = n5147 ? n5146 : n5149;
  /* ../../rtl/core/neorv32_cpu_control.vhd:909:74  */
  assign n5149 = exec[84:53]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:979:16  */
  assign n5177 = exec[18]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:979:37  */
  assign n5178 = ~n5177;
  /* ../../rtl/core/neorv32_cpu_control.vhd:982:48  */
  assign n5180 = exec[23:19]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:982:39  */
  assign n5182 = {27'b000000000000000000000000000, n5180};
  /* ../../rtl/core/neorv32_cpu_control.vhd:979:5  */
  assign n5183 = n5178 ? rf_rs1_i : n5182;
  /* ../../rtl/core/neorv32_cpu_control.vhd:984:17  */
  assign n5184 = exec[17:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:985:45  */
  assign n5185 = csr_rdata | n5183;
  /* ../../rtl/core/neorv32_cpu_control.vhd:985:7  */
  assign n5187 = n5184 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_control.vhd:986:50  */
  assign n5188 = ~n5183;
  /* ../../rtl/core/neorv32_cpu_control.vhd:986:45  */
  assign n5189 = csr_rdata & n5188;
  /* ../../rtl/core/neorv32_cpu_control.vhd:986:7  */
  assign n5191 = n5184 == 2'b11;
  assign n5192 = {n5191, n5187};
  /* ../../rtl/core/neorv32_cpu_control.vhd:984:5  */
  always @*
    case (n5192)
      2'b10: n5193 = n5189;
      2'b01: n5193 = n5185;
      default: n5193 = n5183;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:996:16  */
  assign n5196 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1025:16  */
  assign n5221 = ctrl[163]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:19  */
  assign n5222 = ctrl[176:165]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1029:42  */
  assign n5223 = csr_wdata[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1030:42  */
  assign n5224 = csr_wdata[7]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5232 = csr_wdata[12]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5234 = 1'b0 | n5232;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5236 = csr_wdata[11]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5237 = n5234 | n5236;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1032:42  */
  assign n5238 = csr_wdata[17]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1033:42  */
  assign n5239 = csr_wdata[21]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1028:11  */
  assign n5241 = n5222 == 12'b001100000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1036:38  */
  assign n5242 = csr_wdata[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1037:38  */
  assign n5243 = csr_wdata[7]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1038:38  */
  assign n5244 = csr_wdata[11]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1039:38  */
  assign n5245 = csr_wdata[31:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1035:11  */
  assign n5247 = n5222 == 12'b001100000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1042:35  */
  assign n5248 = csr_wdata[31:2]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1042:49  */
  assign n5250 = {n5248, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1042:66  */
  assign n5251 = csr_wdata[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1042:55  */
  assign n5252 = {n5250, n5251};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1041:11  */
  assign n5254 = n5222 == 12'b001100000101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1044:11  */
  assign n5256 = n5222 == 12'b001100000110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1047:11  */
  assign n5258 = n5222 == 12'b001101000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1051:34  */
  assign n5259 = csr_wdata[31:1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1051:48  */
  assign n5261 = {n5259, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1050:11  */
  assign n5263 = n5222 == 12'b001101000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1054:36  */
  assign n5264 = csr_wdata[31]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1054:52  */
  assign n5265 = csr_wdata[4:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1054:41  */
  assign n5266 = {n5264, n5265};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1053:11  */
  assign n5268 = n5222 == 12'b001101000010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1056:11  */
  assign n5270 = n5222 == 12'b001101000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1060:42  */
  assign n5271 = csr_wdata[2]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1061:42  */
  assign n5272 = csr_wdata[12]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1062:42  */
  assign n5273 = csr_wdata[15]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5281 = csr_wdata[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5283 = 1'b0 | n5281;
  /* ../../rtl/core/neorv32_package.vhd:1220:18  */
  assign n5285 = csr_wdata[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n5286 = n5283 | n5285;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1059:11  */
  assign n5288 = n5222 == 12'b011110110000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1065:11  */
  assign n5293 = n5222 == 12'b011110110001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1068:11  */
  assign n5295 = n5222 == 12'b011110110010;
  assign n5296 = {n5295, n5293, n5288, n5270, n5268, n5263, n5258, n5256, n5254, n5247, n5241};
  assign n5297 = csr[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5298 = n5297;
      11'b01000000000: n5298 = n5297;
      11'b00100000000: n5298 = n5297;
      11'b00010000000: n5298 = n5297;
      11'b00001000000: n5298 = n5297;
      11'b00000100000: n5298 = n5297;
      11'b00000010000: n5298 = n5297;
      11'b00000001000: n5298 = n5297;
      11'b00000000100: n5298 = n5297;
      11'b00000000010: n5298 = n5297;
      11'b00000000001: n5298 = n5223;
      default: n5298 = n5297;
    endcase
  assign n5299 = csr[2]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5300 = n5299;
      11'b01000000000: n5300 = n5299;
      11'b00100000000: n5300 = n5299;
      11'b00010000000: n5300 = n5299;
      11'b00001000000: n5300 = n5299;
      11'b00000100000: n5300 = n5299;
      11'b00000010000: n5300 = n5299;
      11'b00000001000: n5300 = n5299;
      11'b00000000100: n5300 = n5299;
      11'b00000000010: n5300 = n5299;
      11'b00000000001: n5300 = n5224;
      default: n5300 = n5299;
    endcase
  assign n5301 = csr[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5302 = n5301;
      11'b01000000000: n5302 = n5301;
      11'b00100000000: n5302 = n5301;
      11'b00010000000: n5302 = n5301;
      11'b00001000000: n5302 = n5301;
      11'b00000100000: n5302 = n5301;
      11'b00000010000: n5302 = n5301;
      11'b00000001000: n5302 = n5301;
      11'b00000000100: n5302 = n5301;
      11'b00000000010: n5302 = n5301;
      11'b00000000001: n5302 = n5237;
      default: n5302 = n5301;
    endcase
  assign n5303 = csr[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5304 = n5303;
      11'b01000000000: n5304 = n5303;
      11'b00100000000: n5304 = n5303;
      11'b00010000000: n5304 = n5303;
      11'b00001000000: n5304 = n5303;
      11'b00000100000: n5304 = n5303;
      11'b00000010000: n5304 = n5303;
      11'b00000001000: n5304 = n5303;
      11'b00000000100: n5304 = n5303;
      11'b00000000010: n5304 = n5303;
      11'b00000000001: n5304 = n5238;
      default: n5304 = n5303;
    endcase
  assign n5305 = csr[5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5306 = n5305;
      11'b01000000000: n5306 = n5305;
      11'b00100000000: n5306 = n5305;
      11'b00010000000: n5306 = n5305;
      11'b00001000000: n5306 = n5305;
      11'b00000100000: n5306 = n5305;
      11'b00000010000: n5306 = n5305;
      11'b00000001000: n5306 = n5305;
      11'b00000000100: n5306 = n5305;
      11'b00000000010: n5306 = n5305;
      11'b00000000001: n5306 = n5239;
      default: n5306 = n5305;
    endcase
  assign n5307 = csr[6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5308 = n5307;
      11'b01000000000: n5308 = n5307;
      11'b00100000000: n5308 = n5307;
      11'b00010000000: n5308 = n5307;
      11'b00001000000: n5308 = n5307;
      11'b00000100000: n5308 = n5307;
      11'b00000010000: n5308 = n5307;
      11'b00000001000: n5308 = n5307;
      11'b00000000100: n5308 = n5307;
      11'b00000000010: n5308 = n5242;
      11'b00000000001: n5308 = n5307;
      default: n5308 = n5307;
    endcase
  assign n5309 = csr[7]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5310 = n5309;
      11'b01000000000: n5310 = n5309;
      11'b00100000000: n5310 = n5309;
      11'b00010000000: n5310 = n5309;
      11'b00001000000: n5310 = n5309;
      11'b00000100000: n5310 = n5309;
      11'b00000010000: n5310 = n5309;
      11'b00000001000: n5310 = n5309;
      11'b00000000100: n5310 = n5309;
      11'b00000000010: n5310 = n5244;
      11'b00000000001: n5310 = n5309;
      default: n5310 = n5309;
    endcase
  assign n5311 = csr[8]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5312 = n5311;
      11'b01000000000: n5312 = n5311;
      11'b00100000000: n5312 = n5311;
      11'b00010000000: n5312 = n5311;
      11'b00001000000: n5312 = n5311;
      11'b00000100000: n5312 = n5311;
      11'b00000010000: n5312 = n5311;
      11'b00000001000: n5312 = n5311;
      11'b00000000100: n5312 = n5311;
      11'b00000000010: n5312 = n5243;
      11'b00000000001: n5312 = n5311;
      default: n5312 = n5311;
    endcase
  assign n5313 = csr[24:9]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5314 = n5313;
      11'b01000000000: n5314 = n5313;
      11'b00100000000: n5314 = n5313;
      11'b00010000000: n5314 = n5313;
      11'b00001000000: n5314 = n5313;
      11'b00000100000: n5314 = n5313;
      11'b00000010000: n5314 = n5313;
      11'b00000001000: n5314 = n5313;
      11'b00000000100: n5314 = n5313;
      11'b00000000010: n5314 = n5245;
      11'b00000000001: n5314 = n5313;
      default: n5314 = n5313;
    endcase
  assign n5315 = csr[56:25]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5316 = n5315;
      11'b01000000000: n5316 = n5315;
      11'b00100000000: n5316 = n5315;
      11'b00010000000: n5316 = n5315;
      11'b00001000000: n5316 = n5315;
      11'b00000100000: n5316 = n5261;
      11'b00000010000: n5316 = n5315;
      11'b00000001000: n5316 = n5315;
      11'b00000000100: n5316 = n5315;
      11'b00000000010: n5316 = n5315;
      11'b00000000001: n5316 = n5315;
      default: n5316 = n5315;
    endcase
  assign n5317 = csr[62:57]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5318 = n5317;
      11'b01000000000: n5318 = n5317;
      11'b00100000000: n5318 = n5317;
      11'b00010000000: n5318 = n5317;
      11'b00001000000: n5318 = n5266;
      11'b00000100000: n5318 = n5317;
      11'b00000010000: n5318 = n5317;
      11'b00000001000: n5318 = n5317;
      11'b00000000100: n5318 = n5317;
      11'b00000000010: n5318 = n5317;
      11'b00000000001: n5318 = n5317;
      default: n5318 = n5317;
    endcase
  assign n5319 = csr[94:63]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5320 = n5319;
      11'b01000000000: n5320 = n5319;
      11'b00100000000: n5320 = n5319;
      11'b00010000000: n5320 = n5319;
      11'b00001000000: n5320 = n5319;
      11'b00000100000: n5320 = n5319;
      11'b00000010000: n5320 = n5319;
      11'b00000001000: n5320 = n5319;
      11'b00000000100: n5320 = n5252;
      11'b00000000010: n5320 = n5319;
      11'b00000000001: n5320 = n5319;
      default: n5320 = n5319;
    endcase
  assign n5321 = csr[126:95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5322 = n5321;
      11'b01000000000: n5322 = n5321;
      11'b00100000000: n5322 = n5321;
      11'b00010000000: n5322 = csr_wdata;
      11'b00001000000: n5322 = n5321;
      11'b00000100000: n5322 = n5321;
      11'b00000010000: n5322 = n5321;
      11'b00000001000: n5322 = n5321;
      11'b00000000100: n5322 = n5321;
      11'b00000000010: n5322 = n5321;
      11'b00000000001: n5322 = n5321;
      default: n5322 = n5321;
    endcase
  assign n5323 = csr[158:127]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5324 = n5323;
      11'b01000000000: n5324 = n5323;
      11'b00100000000: n5324 = n5323;
      11'b00010000000: n5324 = n5323;
      11'b00001000000: n5324 = n5323;
      11'b00000100000: n5324 = n5323;
      11'b00000010000: n5324 = csr_wdata;
      11'b00000001000: n5324 = n5323;
      11'b00000000100: n5324 = n5323;
      11'b00000000010: n5324 = n5323;
      11'b00000000001: n5324 = n5323;
      default: n5324 = n5323;
    endcase
  assign n5325 = csr[190:159]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5326 = n5325;
      11'b01000000000: n5326 = n5325;
      11'b00100000000: n5326 = n5325;
      11'b00010000000: n5326 = n5325;
      11'b00001000000: n5326 = n5325;
      11'b00000100000: n5326 = n5325;
      11'b00000010000: n5326 = n5325;
      11'b00000001000: n5326 = csr_wdata;
      11'b00000000100: n5326 = n5325;
      11'b00000000010: n5326 = n5325;
      11'b00000000001: n5326 = n5325;
      default: n5326 = n5325;
    endcase
  assign n5327 = csr[191]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5328 = n5327;
      11'b01000000000: n5328 = n5327;
      11'b00100000000: n5328 = n5273;
      11'b00010000000: n5328 = n5327;
      11'b00001000000: n5328 = n5327;
      11'b00000100000: n5328 = n5327;
      11'b00000010000: n5328 = n5327;
      11'b00000001000: n5328 = n5327;
      11'b00000000100: n5328 = n5327;
      11'b00000000010: n5328 = n5327;
      11'b00000000001: n5328 = n5327;
      default: n5328 = n5327;
    endcase
  assign n5329 = csr[192]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5330 = n5329;
      11'b01000000000: n5330 = n5329;
      11'b00100000000: n5330 = n5272;
      11'b00010000000: n5330 = n5329;
      11'b00001000000: n5330 = n5329;
      11'b00000100000: n5330 = n5329;
      11'b00000010000: n5330 = n5329;
      11'b00000001000: n5330 = n5329;
      11'b00000000100: n5330 = n5329;
      11'b00000000010: n5330 = n5329;
      11'b00000000001: n5330 = n5329;
      default: n5330 = n5329;
    endcase
  assign n5331 = csr[193]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5332 = n5331;
      11'b01000000000: n5332 = n5331;
      11'b00100000000: n5332 = n5271;
      11'b00010000000: n5332 = n5331;
      11'b00001000000: n5332 = n5331;
      11'b00000100000: n5332 = n5331;
      11'b00000010000: n5332 = n5331;
      11'b00000001000: n5332 = n5331;
      11'b00000000100: n5332 = n5331;
      11'b00000000010: n5332 = n5331;
      11'b00000000001: n5332 = n5331;
      default: n5332 = n5331;
    endcase
  assign n5333 = csr[194]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1026:9  */
  always @*
    case (n5296)
      11'b10000000000: n5334 = n5333;
      11'b01000000000: n5334 = n5333;
      11'b00100000000: n5334 = n5286;
      11'b00010000000: n5334 = n5333;
      11'b00001000000: n5334 = n5333;
      11'b00000100000: n5334 = n5333;
      11'b00000010000: n5334 = n5333;
      11'b00000001000: n5334 = n5333;
      11'b00000000100: n5334 = n5333;
      11'b00000000010: n5334 = n5333;
      11'b00000000001: n5334 = n5333;
      default: n5334 = n5333;
    endcase
  /* ../../rtl/core/neorv32_cpu_control.vhd:1079:19  */
  assign n5339 = trap[95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1080:24  */
  assign n5340 = debug_ctrl[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1080:28  */
  assign n5341 = ~n5340;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1088:37  */
  assign n5343 = csr[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1090:37  */
  assign n5345 = csr[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1091:43  */
  assign n5346 = trap[61]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1091:59  */
  assign n5347 = trap[59:55]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1091:47  */
  assign n5348 = {n5346, n5347};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1092:40  */
  assign n5349 = trap[93:63]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1092:54  */
  assign n5351 = {n5349, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1093:27  */
  assign n5352 = trap[61]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1093:31  */
  assign n5353 = ~n5352;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1094:29  */
  assign n5354 = trap[57]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1096:32  */
  assign n5355 = trap[56:55]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1096:45  */
  assign n5357 = n5355 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1097:42  */
  assign n5358 = exec[52]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1097:32  */
  assign n5360 = n5358 & 1'b1;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1098:47  */
  assign n5361 = exec[51:36]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1098:40  */
  assign n5363 = {16'b0000000000000000, n5361};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1100:37  */
  assign n5364 = exec[35:4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1097:17  */
  assign n5365 = n5360 ? n5363 : n5364;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1096:15  */
  assign n5367 = n5357 ? n5365 : 32'b00000000000000000000000000000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1094:15  */
  assign n5368 = n5354 ? lsu_mar_i : n5367;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1093:13  */
  assign n5370 = n5353 ? n5368 : 32'b00000000000000000000000000000000;
  assign n5371 = {n5343, n5345, 1'b0, 1'b1};
  assign n5372 = {n5348, n5351};
  assign n5373 = csr[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1080:9  */
  assign n5374 = n5341 ? n5371 : n5373;
  assign n5375 = csr[62:25]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1079:7  */
  assign n5376 = n5399 ? n5372 : n5375;
  assign n5377 = csr[126:95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1079:7  */
  assign n5378 = n5401 ? n5370 : n5377;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1116:19  */
  assign n5379 = trap[96]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1123:32  */
  assign n5380 = csr[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1124:19  */
  assign n5381 = csr[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1124:31  */
  assign n5383 = n5381 != 1'b1;
  assign n5385 = csr[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1124:11  */
  assign n5386 = n5383 ? 1'b0 : n5385;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1128:35  */
  assign n5388 = csr[2]; // extract
  assign n5390 = {n5386, 1'b0, 1'b1, n5388, n5380};
  assign n5391 = csr[4:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1116:7  */
  assign n5392 = n5379 ? n5390 : n5391;
  assign n5393 = n5392[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1079:7  */
  assign n5394 = n5339 ? n5374 : n5393;
  assign n5395 = n5392[4]; // extract
  assign n5396 = csr[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1079:7  */
  assign n5397 = n5339 ? n5396 : n5395;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1079:7  */
  assign n5399 = n5341 & n5339;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1079:7  */
  assign n5401 = n5341 & n5339;
  assign n5402 = {n5397, n5394};
  assign n5403 = {n5334, n5332, n5330, n5328, n5326, n5324, n5322, n5320, n5318, n5316, n5314, n5312, n5310, n5308, n5306, n5304, n5302, n5300, n5298};
  assign n5408 = n5402[4:1]; // extract
  assign n5409 = n5403[3:0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1025:7  */
  assign n5410 = n5221 ? n5409 : n5408;
  assign n5411 = n5403[23:4]; // extract
  assign n5412 = csr[24:5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1025:7  */
  assign n5413 = n5221 ? n5411 : n5412;
  assign n5414 = n5403[61:24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1025:7  */
  assign n5415 = n5221 ? n5414 : n5376;
  assign n5416 = n5403[93:62]; // extract
  assign n5417 = csr[94:63]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1025:7  */
  assign n5418 = n5221 ? n5416 : n5417;
  assign n5419 = n5403[125:94]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1025:7  */
  assign n5420 = n5221 ? n5419 : n5378;
  assign n5421 = n5403[193:126]; // extract
  assign n5422 = csr[194:127]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1025:7  */
  assign n5423 = n5221 ? n5421 : n5422;
  assign n5432 = n5410[1:0]; // extract
  assign n5435 = n5413[19:1]; // extract
  assign n5442 = n5423[31:0]; // extract
  assign n5451 = {32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 3'b000, 1'b1, 1'b0, 1'b0, 1'b0, 32'b00000000000000000000000000000000, n5442, n5420, n5418, n5415, n5435, 1'b0, 1'b0, 1'b1, n5432, 1'b1};
  assign n5453 = {32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 3'b000, 1'b0, 1'b0, 1'b0, 1'b0, 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 32'b00000000000000000000000000000000, 6'b000000, 32'b00000000000000000000000000000000, 16'b0000000000000000, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b1};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1178:16  */
  assign n5457 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1182:16  */
  assign n5459 = ctrl[164]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:19  */
  assign n5460 = ctrl[176:165]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1189:34  */
  assign n5461 = csr[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1190:34  */
  assign n5462 = csr[2]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1191:34  */
  assign n5463 = csr[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1192:34  */
  assign n5464 = csr[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1193:34  */
  assign n5465 = csr[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1194:34  */
  assign n5466 = csr[5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1194:45  */
  assign n5469 = n5466 & 1'b0;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1188:11  */
  assign n5471 = n5460 == 12'b001100000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1196:11  */
  assign n5489 = n5460 == 12'b001100000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1208:34  */
  assign n5490 = csr[6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1209:34  */
  assign n5491 = csr[8]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1210:34  */
  assign n5492 = csr[7]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1211:44  */
  assign n5493 = csr[24:9]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1207:11  */
  assign n5495 = n5460 == 12'b001100000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1214:30  */
  assign n5496 = csr[94:63]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1213:11  */
  assign n5498 = n5460 == 12'b001100000101;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1216:11  */
  assign n5500 = n5460 == 12'b001100000110;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1225:30  */
  assign n5501 = csr[158:127]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1224:11  */
  assign n5503 = n5460 == 12'b001101000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1228:34  */
  assign n5504 = csr[56:26]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1228:48  */
  assign n5506 = {n5504, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1227:11  */
  assign n5508 = n5460 == 12'b001101000001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1231:40  */
  assign n5509 = csr[62]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1232:48  */
  assign n5510 = csr[61:57]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1230:11  */
  assign n5512 = n5460 == 12'b001101000010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1235:30  */
  assign n5513 = csr[126:95]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1234:11  */
  assign n5515 = n5460 == 12'b001101000011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1238:42  */
  assign n5516 = trap[13]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1239:42  */
  assign n5517 = trap[14]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1240:42  */
  assign n5518 = trap[15]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1241:52  */
  assign n5519 = trap[31:16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1237:11  */
  assign n5521 = n5460 == 12'b001101000100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1246:11  */
  assign n5523 = n5460 == 12'b111100010001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1247:11  */
  assign n5525 = n5460 == 12'b111100010010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1248:11  */
  assign n5527 = n5460 == 12'b111100010011;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1249:11  */
  assign n5529 = n5460 == 12'b111100010100;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1254:11  */
  assign n5531 = n5460 == 12'b011110110000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1255:11  */
  assign n5533 = n5460 == 12'b011110110001;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1256:11  */
  assign n5535 = n5460 == 12'b011110110010;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1261:11  */
  assign n5599 = n5460 == 12'b111111000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1295:11  */
  assign n5603 = n5460 == 12'b111111000001;
  assign n5604 = {n5603, n5599, n5535, n5533, n5531, n5529, n5527, n5525, n5523, n5521, n5515, n5512, n5508, n5503, n5500, n5498, n5495, n5489, n5471};
  assign n5605 = n5496[0]; // extract
  assign n5606 = n5501[0]; // extract
  assign n5607 = n5506[0]; // extract
  assign n5608 = n5510[0]; // extract
  assign n5609 = n5513[0]; // extract
  assign n5614 = xcsr_rdata_i[0]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5616 = 1'b0;
      19'b0100000000000000000: n5616 = 1'b1;
      19'b0010000000000000000: n5616 = 1'b0;
      19'b0001000000000000000: n5616 = 1'b0;
      19'b0000100000000000000: n5616 = 1'b0;
      19'b0000010000000000000: n5616 = 1'b0;
      19'b0000001000000000000: n5616 = 1'b1;
      19'b0000000100000000000: n5616 = 1'b1;
      19'b0000000010000000000: n5616 = 1'b0;
      19'b0000000001000000000: n5616 = 1'b0;
      19'b0000000000100000000: n5616 = n5609;
      19'b0000000000010000000: n5616 = n5608;
      19'b0000000000001000000: n5616 = n5607;
      19'b0000000000000100000: n5616 = n5606;
      19'b0000000000000010000: n5616 = 1'b0;
      19'b0000000000000001000: n5616 = n5605;
      19'b0000000000000000100: n5616 = 1'b0;
      19'b0000000000000000010: n5616 = 1'b0;
      19'b0000000000000000001: n5616 = 1'b0;
      default: n5616 = n5614;
    endcase
  assign n5617 = n5496[1]; // extract
  assign n5618 = n5501[1]; // extract
  assign n5619 = n5506[1]; // extract
  assign n5620 = n5510[1]; // extract
  assign n5621 = n5513[1]; // extract
  assign n5626 = xcsr_rdata_i[1]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5628 = 1'b0;
      19'b0100000000000000000: n5628 = 1'b1;
      19'b0010000000000000000: n5628 = 1'b0;
      19'b0001000000000000000: n5628 = 1'b0;
      19'b0000100000000000000: n5628 = 1'b0;
      19'b0000010000000000000: n5628 = 1'b0;
      19'b0000001000000000000: n5628 = 1'b0;
      19'b0000000100000000000: n5628 = 1'b1;
      19'b0000000010000000000: n5628 = 1'b0;
      19'b0000000001000000000: n5628 = 1'b0;
      19'b0000000000100000000: n5628 = n5621;
      19'b0000000000010000000: n5628 = n5620;
      19'b0000000000001000000: n5628 = n5619;
      19'b0000000000000100000: n5628 = n5618;
      19'b0000000000000010000: n5628 = 1'b0;
      19'b0000000000000001000: n5628 = n5617;
      19'b0000000000000000100: n5628 = 1'b0;
      19'b0000000000000000010: n5628 = 1'b0;
      19'b0000000000000000001: n5628 = 1'b0;
      default: n5628 = n5626;
    endcase
  assign n5629 = n5496[2]; // extract
  assign n5630 = n5501[2]; // extract
  assign n5631 = n5506[2]; // extract
  assign n5632 = n5510[2]; // extract
  assign n5633 = n5513[2]; // extract
  assign n5638 = xcsr_rdata_i[2]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5640 = 1'b0;
      19'b0100000000000000000: n5640 = 1'b0;
      19'b0010000000000000000: n5640 = 1'b0;
      19'b0001000000000000000: n5640 = 1'b0;
      19'b0000100000000000000: n5640 = 1'b0;
      19'b0000010000000000000: n5640 = 1'b0;
      19'b0000001000000000000: n5640 = 1'b0;
      19'b0000000100000000000: n5640 = 1'b0;
      19'b0000000010000000000: n5640 = 1'b0;
      19'b0000000001000000000: n5640 = 1'b0;
      19'b0000000000100000000: n5640 = n5633;
      19'b0000000000010000000: n5640 = n5632;
      19'b0000000000001000000: n5640 = n5631;
      19'b0000000000000100000: n5640 = n5630;
      19'b0000000000000010000: n5640 = 1'b0;
      19'b0000000000000001000: n5640 = n5629;
      19'b0000000000000000100: n5640 = 1'b0;
      19'b0000000000000000010: n5640 = 1'b1;
      19'b0000000000000000001: n5640 = 1'b0;
      default: n5640 = n5638;
    endcase
  assign n5641 = n5496[3]; // extract
  assign n5642 = n5501[3]; // extract
  assign n5643 = n5506[3]; // extract
  assign n5644 = n5510[3]; // extract
  assign n5645 = n5513[3]; // extract
  assign n5650 = xcsr_rdata_i[3]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5652 = 1'b0;
      19'b0100000000000000000: n5652 = 1'b0;
      19'b0010000000000000000: n5652 = 1'b0;
      19'b0001000000000000000: n5652 = 1'b0;
      19'b0000100000000000000: n5652 = 1'b0;
      19'b0000010000000000000: n5652 = 1'b0;
      19'b0000001000000000000: n5652 = 1'b0;
      19'b0000000100000000000: n5652 = 1'b0;
      19'b0000000010000000000: n5652 = 1'b0;
      19'b0000000001000000000: n5652 = n5516;
      19'b0000000000100000000: n5652 = n5645;
      19'b0000000000010000000: n5652 = n5644;
      19'b0000000000001000000: n5652 = n5643;
      19'b0000000000000100000: n5652 = n5642;
      19'b0000000000000010000: n5652 = 1'b0;
      19'b0000000000000001000: n5652 = n5641;
      19'b0000000000000000100: n5652 = n5490;
      19'b0000000000000000010: n5652 = 1'b0;
      19'b0000000000000000001: n5652 = n5461;
      default: n5652 = n5650;
    endcase
  assign n5653 = n5496[4]; // extract
  assign n5654 = n5501[4]; // extract
  assign n5655 = n5506[4]; // extract
  assign n5656 = n5510[4]; // extract
  assign n5657 = n5513[4]; // extract
  assign n5662 = xcsr_rdata_i[4]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5664 = 1'b0;
      19'b0100000000000000000: n5664 = 1'b0;
      19'b0010000000000000000: n5664 = 1'b0;
      19'b0001000000000000000: n5664 = 1'b0;
      19'b0000100000000000000: n5664 = 1'b0;
      19'b0000010000000000000: n5664 = 1'b0;
      19'b0000001000000000000: n5664 = 1'b0;
      19'b0000000100000000000: n5664 = 1'b1;
      19'b0000000010000000000: n5664 = 1'b0;
      19'b0000000001000000000: n5664 = 1'b0;
      19'b0000000000100000000: n5664 = n5657;
      19'b0000000000010000000: n5664 = n5656;
      19'b0000000000001000000: n5664 = n5655;
      19'b0000000000000100000: n5664 = n5654;
      19'b0000000000000010000: n5664 = 1'b0;
      19'b0000000000000001000: n5664 = n5653;
      19'b0000000000000000100: n5664 = 1'b0;
      19'b0000000000000000010: n5664 = 1'b0;
      19'b0000000000000000001: n5664 = 1'b0;
      default: n5664 = n5662;
    endcase
  assign n5665 = n5496[5]; // extract
  assign n5666 = n5501[5]; // extract
  assign n5667 = n5506[5]; // extract
  assign n5668 = n5513[5]; // extract
  assign n5673 = xcsr_rdata_i[5]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5675 = 1'b0;
      19'b0100000000000000000: n5675 = 1'b0;
      19'b0010000000000000000: n5675 = 1'b0;
      19'b0001000000000000000: n5675 = 1'b0;
      19'b0000100000000000000: n5675 = 1'b0;
      19'b0000010000000000000: n5675 = 1'b0;
      19'b0000001000000000000: n5675 = 1'b0;
      19'b0000000100000000000: n5675 = 1'b0;
      19'b0000000010000000000: n5675 = 1'b0;
      19'b0000000001000000000: n5675 = 1'b0;
      19'b0000000000100000000: n5675 = n5668;
      19'b0000000000010000000: n5675 = 1'b0;
      19'b0000000000001000000: n5675 = n5667;
      19'b0000000000000100000: n5675 = n5666;
      19'b0000000000000010000: n5675 = 1'b0;
      19'b0000000000000001000: n5675 = n5665;
      19'b0000000000000000100: n5675 = 1'b0;
      19'b0000000000000000010: n5675 = 1'b0;
      19'b0000000000000000001: n5675 = 1'b0;
      default: n5675 = n5673;
    endcase
  assign n5676 = n5496[6]; // extract
  assign n5677 = n5501[6]; // extract
  assign n5678 = n5506[6]; // extract
  assign n5679 = n5513[6]; // extract
  assign n5684 = xcsr_rdata_i[6]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5686 = 1'b0;
      19'b0100000000000000000: n5686 = 1'b0;
      19'b0010000000000000000: n5686 = 1'b0;
      19'b0001000000000000000: n5686 = 1'b0;
      19'b0000100000000000000: n5686 = 1'b0;
      19'b0000010000000000000: n5686 = 1'b0;
      19'b0000001000000000000: n5686 = 1'b0;
      19'b0000000100000000000: n5686 = 1'b0;
      19'b0000000010000000000: n5686 = 1'b0;
      19'b0000000001000000000: n5686 = 1'b0;
      19'b0000000000100000000: n5686 = n5679;
      19'b0000000000010000000: n5686 = 1'b0;
      19'b0000000000001000000: n5686 = n5678;
      19'b0000000000000100000: n5686 = n5677;
      19'b0000000000000010000: n5686 = 1'b0;
      19'b0000000000000001000: n5686 = n5676;
      19'b0000000000000000100: n5686 = 1'b0;
      19'b0000000000000000010: n5686 = 1'b0;
      19'b0000000000000000001: n5686 = 1'b0;
      default: n5686 = n5684;
    endcase
  assign n5687 = n5496[7]; // extract
  assign n5688 = n5501[7]; // extract
  assign n5689 = n5506[7]; // extract
  assign n5690 = n5513[7]; // extract
  assign n5695 = xcsr_rdata_i[7]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5697 = 1'b0;
      19'b0100000000000000000: n5697 = 1'b1;
      19'b0010000000000000000: n5697 = 1'b0;
      19'b0001000000000000000: n5697 = 1'b0;
      19'b0000100000000000000: n5697 = 1'b0;
      19'b0000010000000000000: n5697 = 1'b0;
      19'b0000001000000000000: n5697 = 1'b0;
      19'b0000000100000000000: n5697 = 1'b0;
      19'b0000000010000000000: n5697 = 1'b0;
      19'b0000000001000000000: n5697 = n5517;
      19'b0000000000100000000: n5697 = n5690;
      19'b0000000000010000000: n5697 = 1'b0;
      19'b0000000000001000000: n5697 = n5689;
      19'b0000000000000100000: n5697 = n5688;
      19'b0000000000000010000: n5697 = 1'b0;
      19'b0000000000000001000: n5697 = n5687;
      19'b0000000000000000100: n5697 = n5491;
      19'b0000000000000000010: n5697 = 1'b0;
      19'b0000000000000000001: n5697 = n5462;
      default: n5697 = n5695;
    endcase
  assign n5698 = n5496[8]; // extract
  assign n5699 = n5501[8]; // extract
  assign n5700 = n5506[8]; // extract
  assign n5701 = n5513[8]; // extract
  assign n5706 = xcsr_rdata_i[8]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5708 = 1'b0;
      19'b0100000000000000000: n5708 = 1'b0;
      19'b0010000000000000000: n5708 = 1'b0;
      19'b0001000000000000000: n5708 = 1'b0;
      19'b0000100000000000000: n5708 = 1'b0;
      19'b0000010000000000000: n5708 = 1'b0;
      19'b0000001000000000000: n5708 = 1'b1;
      19'b0000000100000000000: n5708 = 1'b0;
      19'b0000000010000000000: n5708 = 1'b0;
      19'b0000000001000000000: n5708 = 1'b0;
      19'b0000000000100000000: n5708 = n5701;
      19'b0000000000010000000: n5708 = 1'b0;
      19'b0000000000001000000: n5708 = n5700;
      19'b0000000000000100000: n5708 = n5699;
      19'b0000000000000010000: n5708 = 1'b0;
      19'b0000000000000001000: n5708 = n5698;
      19'b0000000000000000100: n5708 = 1'b0;
      19'b0000000000000000010: n5708 = 1'b1;
      19'b0000000000000000001: n5708 = 1'b0;
      default: n5708 = n5706;
    endcase
  assign n5709 = n5496[9]; // extract
  assign n5710 = n5501[9]; // extract
  assign n5711 = n5506[9]; // extract
  assign n5712 = n5513[9]; // extract
  assign n5717 = xcsr_rdata_i[9]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5719 = 1'b0;
      19'b0100000000000000000: n5719 = 1'b0;
      19'b0010000000000000000: n5719 = 1'b0;
      19'b0001000000000000000: n5719 = 1'b0;
      19'b0000100000000000000: n5719 = 1'b0;
      19'b0000010000000000000: n5719 = 1'b0;
      19'b0000001000000000000: n5719 = 1'b0;
      19'b0000000100000000000: n5719 = 1'b0;
      19'b0000000010000000000: n5719 = 1'b0;
      19'b0000000001000000000: n5719 = 1'b0;
      19'b0000000000100000000: n5719 = n5712;
      19'b0000000000010000000: n5719 = 1'b0;
      19'b0000000000001000000: n5719 = n5711;
      19'b0000000000000100000: n5719 = n5710;
      19'b0000000000000010000: n5719 = 1'b0;
      19'b0000000000000001000: n5719 = n5709;
      19'b0000000000000000100: n5719 = 1'b0;
      19'b0000000000000000010: n5719 = 1'b0;
      19'b0000000000000000001: n5719 = 1'b0;
      default: n5719 = n5717;
    endcase
  assign n5720 = n5496[10]; // extract
  assign n5721 = n5501[10]; // extract
  assign n5722 = n5506[10]; // extract
  assign n5723 = n5513[10]; // extract
  assign n5728 = xcsr_rdata_i[10]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5730 = 1'b0;
      19'b0100000000000000000: n5730 = 1'b0;
      19'b0010000000000000000: n5730 = 1'b0;
      19'b0001000000000000000: n5730 = 1'b0;
      19'b0000100000000000000: n5730 = 1'b0;
      19'b0000010000000000000: n5730 = 1'b0;
      19'b0000001000000000000: n5730 = 1'b0;
      19'b0000000100000000000: n5730 = 1'b0;
      19'b0000000010000000000: n5730 = 1'b0;
      19'b0000000001000000000: n5730 = 1'b0;
      19'b0000000000100000000: n5730 = n5723;
      19'b0000000000010000000: n5730 = 1'b0;
      19'b0000000000001000000: n5730 = n5722;
      19'b0000000000000100000: n5730 = n5721;
      19'b0000000000000010000: n5730 = 1'b0;
      19'b0000000000000001000: n5730 = n5720;
      19'b0000000000000000100: n5730 = 1'b0;
      19'b0000000000000000010: n5730 = 1'b0;
      19'b0000000000000000001: n5730 = 1'b0;
      default: n5730 = n5728;
    endcase
  assign n5731 = n5496[11]; // extract
  assign n5732 = n5501[11]; // extract
  assign n5733 = n5506[11]; // extract
  assign n5734 = n5513[11]; // extract
  assign n5739 = xcsr_rdata_i[11]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5741 = 1'b0;
      19'b0100000000000000000: n5741 = 1'b0;
      19'b0010000000000000000: n5741 = 1'b0;
      19'b0001000000000000000: n5741 = 1'b0;
      19'b0000100000000000000: n5741 = 1'b0;
      19'b0000010000000000000: n5741 = 1'b0;
      19'b0000001000000000000: n5741 = 1'b0;
      19'b0000000100000000000: n5741 = 1'b0;
      19'b0000000010000000000: n5741 = 1'b0;
      19'b0000000001000000000: n5741 = n5518;
      19'b0000000000100000000: n5741 = n5734;
      19'b0000000000010000000: n5741 = 1'b0;
      19'b0000000000001000000: n5741 = n5733;
      19'b0000000000000100000: n5741 = n5732;
      19'b0000000000000010000: n5741 = 1'b0;
      19'b0000000000000001000: n5741 = n5731;
      19'b0000000000000000100: n5741 = n5492;
      19'b0000000000000000010: n5741 = 1'b0;
      19'b0000000000000000001: n5741 = n5463;
      default: n5741 = n5739;
    endcase
  assign n5742 = n5496[12]; // extract
  assign n5743 = n5501[12]; // extract
  assign n5744 = n5506[12]; // extract
  assign n5745 = n5513[12]; // extract
  assign n5750 = xcsr_rdata_i[12]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5752 = 1'b0;
      19'b0100000000000000000: n5752 = 1'b0;
      19'b0010000000000000000: n5752 = 1'b0;
      19'b0001000000000000000: n5752 = 1'b0;
      19'b0000100000000000000: n5752 = 1'b0;
      19'b0000010000000000000: n5752 = 1'b0;
      19'b0000001000000000000: n5752 = 1'b0;
      19'b0000000100000000000: n5752 = 1'b0;
      19'b0000000010000000000: n5752 = 1'b0;
      19'b0000000001000000000: n5752 = 1'b0;
      19'b0000000000100000000: n5752 = n5745;
      19'b0000000000010000000: n5752 = 1'b0;
      19'b0000000000001000000: n5752 = n5744;
      19'b0000000000000100000: n5752 = n5743;
      19'b0000000000000010000: n5752 = 1'b0;
      19'b0000000000000001000: n5752 = n5742;
      19'b0000000000000000100: n5752 = 1'b0;
      19'b0000000000000000010: n5752 = 1'b1;
      19'b0000000000000000001: n5752 = n5464;
      default: n5752 = n5750;
    endcase
  assign n5753 = n5496[13]; // extract
  assign n5754 = n5501[13]; // extract
  assign n5755 = n5506[13]; // extract
  assign n5756 = n5513[13]; // extract
  assign n5761 = xcsr_rdata_i[13]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5763 = 1'b0;
      19'b0100000000000000000: n5763 = 1'b0;
      19'b0010000000000000000: n5763 = 1'b0;
      19'b0001000000000000000: n5763 = 1'b0;
      19'b0000100000000000000: n5763 = 1'b0;
      19'b0000010000000000000: n5763 = 1'b0;
      19'b0000001000000000000: n5763 = 1'b0;
      19'b0000000100000000000: n5763 = 1'b0;
      19'b0000000010000000000: n5763 = 1'b0;
      19'b0000000001000000000: n5763 = 1'b0;
      19'b0000000000100000000: n5763 = n5756;
      19'b0000000000010000000: n5763 = 1'b0;
      19'b0000000000001000000: n5763 = n5755;
      19'b0000000000000100000: n5763 = n5754;
      19'b0000000000000010000: n5763 = 1'b0;
      19'b0000000000000001000: n5763 = n5753;
      19'b0000000000000000100: n5763 = 1'b0;
      19'b0000000000000000010: n5763 = 1'b0;
      19'b0000000000000000001: n5763 = 1'b0;
      default: n5763 = n5761;
    endcase
  assign n5764 = n5496[14]; // extract
  assign n5765 = n5501[14]; // extract
  assign n5766 = n5506[14]; // extract
  assign n5767 = n5513[14]; // extract
  assign n5772 = xcsr_rdata_i[14]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5774 = 1'b0;
      19'b0100000000000000000: n5774 = 1'b0;
      19'b0010000000000000000: n5774 = 1'b0;
      19'b0001000000000000000: n5774 = 1'b0;
      19'b0000100000000000000: n5774 = 1'b0;
      19'b0000010000000000000: n5774 = 1'b0;
      19'b0000001000000000000: n5774 = 1'b0;
      19'b0000000100000000000: n5774 = 1'b0;
      19'b0000000010000000000: n5774 = 1'b0;
      19'b0000000001000000000: n5774 = 1'b0;
      19'b0000000000100000000: n5774 = n5767;
      19'b0000000000010000000: n5774 = 1'b0;
      19'b0000000000001000000: n5774 = n5766;
      19'b0000000000000100000: n5774 = n5765;
      19'b0000000000000010000: n5774 = 1'b0;
      19'b0000000000000001000: n5774 = n5764;
      19'b0000000000000000100: n5774 = 1'b0;
      19'b0000000000000000010: n5774 = 1'b0;
      19'b0000000000000000001: n5774 = 1'b0;
      default: n5774 = n5772;
    endcase
  assign n5775 = n5496[15]; // extract
  assign n5776 = n5501[15]; // extract
  assign n5777 = n5506[15]; // extract
  assign n5778 = n5513[15]; // extract
  assign n5783 = xcsr_rdata_i[15]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5785 = 1'b0;
      19'b0100000000000000000: n5785 = 1'b0;
      19'b0010000000000000000: n5785 = 1'b0;
      19'b0001000000000000000: n5785 = 1'b0;
      19'b0000100000000000000: n5785 = 1'b0;
      19'b0000010000000000000: n5785 = 1'b0;
      19'b0000001000000000000: n5785 = 1'b0;
      19'b0000000100000000000: n5785 = 1'b0;
      19'b0000000010000000000: n5785 = 1'b0;
      19'b0000000001000000000: n5785 = 1'b0;
      19'b0000000000100000000: n5785 = n5778;
      19'b0000000000010000000: n5785 = 1'b0;
      19'b0000000000001000000: n5785 = n5777;
      19'b0000000000000100000: n5785 = n5776;
      19'b0000000000000010000: n5785 = 1'b0;
      19'b0000000000000001000: n5785 = n5775;
      19'b0000000000000000100: n5785 = 1'b0;
      19'b0000000000000000010: n5785 = 1'b0;
      19'b0000000000000000001: n5785 = 1'b0;
      default: n5785 = n5783;
    endcase
  assign n5786 = n5493[0]; // extract
  assign n5787 = n5496[16]; // extract
  assign n5788 = n5501[16]; // extract
  assign n5789 = n5506[16]; // extract
  assign n5790 = n5513[16]; // extract
  assign n5791 = n5519[0]; // extract
  assign n5796 = xcsr_rdata_i[16]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5798 = 1'b0;
      19'b0100000000000000000: n5798 = 1'b0;
      19'b0010000000000000000: n5798 = 1'b0;
      19'b0001000000000000000: n5798 = 1'b0;
      19'b0000100000000000000: n5798 = 1'b0;
      19'b0000010000000000000: n5798 = 1'b0;
      19'b0000001000000000000: n5798 = 1'b1;
      19'b0000000100000000000: n5798 = 1'b0;
      19'b0000000010000000000: n5798 = 1'b0;
      19'b0000000001000000000: n5798 = n5791;
      19'b0000000000100000000: n5798 = n5790;
      19'b0000000000010000000: n5798 = 1'b0;
      19'b0000000000001000000: n5798 = n5789;
      19'b0000000000000100000: n5798 = n5788;
      19'b0000000000000010000: n5798 = 1'b0;
      19'b0000000000000001000: n5798 = n5787;
      19'b0000000000000000100: n5798 = n5786;
      19'b0000000000000000010: n5798 = 1'b0;
      19'b0000000000000000001: n5798 = 1'b0;
      default: n5798 = n5796;
    endcase
  assign n5799 = n5493[1]; // extract
  assign n5800 = n5496[17]; // extract
  assign n5801 = n5501[17]; // extract
  assign n5802 = n5506[17]; // extract
  assign n5803 = n5513[17]; // extract
  assign n5804 = n5519[1]; // extract
  assign n5809 = xcsr_rdata_i[17]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5811 = 1'b0;
      19'b0100000000000000000: n5811 = 1'b0;
      19'b0010000000000000000: n5811 = 1'b0;
      19'b0001000000000000000: n5811 = 1'b0;
      19'b0000100000000000000: n5811 = 1'b0;
      19'b0000010000000000000: n5811 = 1'b0;
      19'b0000001000000000000: n5811 = 1'b1;
      19'b0000000100000000000: n5811 = 1'b0;
      19'b0000000010000000000: n5811 = 1'b0;
      19'b0000000001000000000: n5811 = n5804;
      19'b0000000000100000000: n5811 = n5803;
      19'b0000000000010000000: n5811 = 1'b0;
      19'b0000000000001000000: n5811 = n5802;
      19'b0000000000000100000: n5811 = n5801;
      19'b0000000000000010000: n5811 = 1'b0;
      19'b0000000000000001000: n5811 = n5800;
      19'b0000000000000000100: n5811 = n5799;
      19'b0000000000000000010: n5811 = 1'b0;
      19'b0000000000000000001: n5811 = n5465;
      default: n5811 = n5809;
    endcase
  assign n5812 = n5493[2]; // extract
  assign n5813 = n5496[18]; // extract
  assign n5814 = n5501[18]; // extract
  assign n5815 = n5506[18]; // extract
  assign n5816 = n5513[18]; // extract
  assign n5817 = n5519[2]; // extract
  assign n5822 = xcsr_rdata_i[18]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5824 = 1'b0;
      19'b0100000000000000000: n5824 = 1'b0;
      19'b0010000000000000000: n5824 = 1'b0;
      19'b0001000000000000000: n5824 = 1'b0;
      19'b0000100000000000000: n5824 = 1'b0;
      19'b0000010000000000000: n5824 = 1'b0;
      19'b0000001000000000000: n5824 = 1'b0;
      19'b0000000100000000000: n5824 = 1'b0;
      19'b0000000010000000000: n5824 = 1'b0;
      19'b0000000001000000000: n5824 = n5817;
      19'b0000000000100000000: n5824 = n5816;
      19'b0000000000010000000: n5824 = 1'b0;
      19'b0000000000001000000: n5824 = n5815;
      19'b0000000000000100000: n5824 = n5814;
      19'b0000000000000010000: n5824 = 1'b0;
      19'b0000000000000001000: n5824 = n5813;
      19'b0000000000000000100: n5824 = n5812;
      19'b0000000000000000010: n5824 = 1'b0;
      19'b0000000000000000001: n5824 = 1'b0;
      default: n5824 = n5822;
    endcase
  assign n5825 = n5493[3]; // extract
  assign n5826 = n5496[19]; // extract
  assign n5827 = n5501[19]; // extract
  assign n5828 = n5506[19]; // extract
  assign n5829 = n5513[19]; // extract
  assign n5830 = n5519[3]; // extract
  assign n5835 = xcsr_rdata_i[19]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5837 = 1'b0;
      19'b0100000000000000000: n5837 = 1'b0;
      19'b0010000000000000000: n5837 = 1'b0;
      19'b0001000000000000000: n5837 = 1'b0;
      19'b0000100000000000000: n5837 = 1'b0;
      19'b0000010000000000000: n5837 = 1'b0;
      19'b0000001000000000000: n5837 = 1'b0;
      19'b0000000100000000000: n5837 = 1'b0;
      19'b0000000010000000000: n5837 = 1'b0;
      19'b0000000001000000000: n5837 = n5830;
      19'b0000000000100000000: n5837 = n5829;
      19'b0000000000010000000: n5837 = 1'b0;
      19'b0000000000001000000: n5837 = n5828;
      19'b0000000000000100000: n5837 = n5827;
      19'b0000000000000010000: n5837 = 1'b0;
      19'b0000000000000001000: n5837 = n5826;
      19'b0000000000000000100: n5837 = n5825;
      19'b0000000000000000010: n5837 = 1'b0;
      19'b0000000000000000001: n5837 = 1'b0;
      default: n5837 = n5835;
    endcase
  assign n5838 = n5493[4]; // extract
  assign n5839 = n5496[20]; // extract
  assign n5840 = n5501[20]; // extract
  assign n5841 = n5506[20]; // extract
  assign n5842 = n5513[20]; // extract
  assign n5843 = n5519[4]; // extract
  assign n5848 = xcsr_rdata_i[20]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5850 = 1'b0;
      19'b0100000000000000000: n5850 = 1'b0;
      19'b0010000000000000000: n5850 = 1'b0;
      19'b0001000000000000000: n5850 = 1'b0;
      19'b0000100000000000000: n5850 = 1'b0;
      19'b0000010000000000000: n5850 = 1'b0;
      19'b0000001000000000000: n5850 = 1'b1;
      19'b0000000100000000000: n5850 = 1'b0;
      19'b0000000010000000000: n5850 = 1'b0;
      19'b0000000001000000000: n5850 = n5843;
      19'b0000000000100000000: n5850 = n5842;
      19'b0000000000010000000: n5850 = 1'b0;
      19'b0000000000001000000: n5850 = n5841;
      19'b0000000000000100000: n5850 = n5840;
      19'b0000000000000010000: n5850 = 1'b0;
      19'b0000000000000001000: n5850 = n5839;
      19'b0000000000000000100: n5850 = n5838;
      19'b0000000000000000010: n5850 = 1'b0;
      19'b0000000000000000001: n5850 = 1'b0;
      default: n5850 = n5848;
    endcase
  assign n5851 = n5493[5]; // extract
  assign n5852 = n5496[21]; // extract
  assign n5853 = n5501[21]; // extract
  assign n5854 = n5506[21]; // extract
  assign n5855 = n5513[21]; // extract
  assign n5856 = n5519[5]; // extract
  assign n5861 = xcsr_rdata_i[21]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5863 = 1'b0;
      19'b0100000000000000000: n5863 = 1'b0;
      19'b0010000000000000000: n5863 = 1'b0;
      19'b0001000000000000000: n5863 = 1'b0;
      19'b0000100000000000000: n5863 = 1'b0;
      19'b0000010000000000000: n5863 = 1'b0;
      19'b0000001000000000000: n5863 = 1'b0;
      19'b0000000100000000000: n5863 = 1'b0;
      19'b0000000010000000000: n5863 = 1'b0;
      19'b0000000001000000000: n5863 = n5856;
      19'b0000000000100000000: n5863 = n5855;
      19'b0000000000010000000: n5863 = 1'b0;
      19'b0000000000001000000: n5863 = n5854;
      19'b0000000000000100000: n5863 = n5853;
      19'b0000000000000010000: n5863 = 1'b0;
      19'b0000000000000001000: n5863 = n5852;
      19'b0000000000000000100: n5863 = n5851;
      19'b0000000000000000010: n5863 = 1'b0;
      19'b0000000000000000001: n5863 = n5469;
      default: n5863 = n5861;
    endcase
  assign n5864 = n5493[6]; // extract
  assign n5865 = n5496[22]; // extract
  assign n5866 = n5501[22]; // extract
  assign n5867 = n5506[22]; // extract
  assign n5868 = n5513[22]; // extract
  assign n5869 = n5519[6]; // extract
  assign n5874 = xcsr_rdata_i[22]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5876 = 1'b0;
      19'b0100000000000000000: n5876 = 1'b0;
      19'b0010000000000000000: n5876 = 1'b0;
      19'b0001000000000000000: n5876 = 1'b0;
      19'b0000100000000000000: n5876 = 1'b0;
      19'b0000010000000000000: n5876 = 1'b0;
      19'b0000001000000000000: n5876 = 1'b0;
      19'b0000000100000000000: n5876 = 1'b0;
      19'b0000000010000000000: n5876 = 1'b0;
      19'b0000000001000000000: n5876 = n5869;
      19'b0000000000100000000: n5876 = n5868;
      19'b0000000000010000000: n5876 = 1'b0;
      19'b0000000000001000000: n5876 = n5867;
      19'b0000000000000100000: n5876 = n5866;
      19'b0000000000000010000: n5876 = 1'b0;
      19'b0000000000000001000: n5876 = n5865;
      19'b0000000000000000100: n5876 = n5864;
      19'b0000000000000000010: n5876 = 1'b0;
      19'b0000000000000000001: n5876 = 1'b0;
      default: n5876 = n5874;
    endcase
  assign n5877 = n5493[7]; // extract
  assign n5878 = n5496[23]; // extract
  assign n5879 = n5501[23]; // extract
  assign n5880 = n5506[23]; // extract
  assign n5881 = n5513[23]; // extract
  assign n5882 = n5519[7]; // extract
  assign n5887 = xcsr_rdata_i[23]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5889 = 1'b0;
      19'b0100000000000000000: n5889 = 1'b0;
      19'b0010000000000000000: n5889 = 1'b0;
      19'b0001000000000000000: n5889 = 1'b0;
      19'b0000100000000000000: n5889 = 1'b0;
      19'b0000010000000000000: n5889 = 1'b0;
      19'b0000001000000000000: n5889 = 1'b0;
      19'b0000000100000000000: n5889 = 1'b0;
      19'b0000000010000000000: n5889 = 1'b0;
      19'b0000000001000000000: n5889 = n5882;
      19'b0000000000100000000: n5889 = n5881;
      19'b0000000000010000000: n5889 = 1'b0;
      19'b0000000000001000000: n5889 = n5880;
      19'b0000000000000100000: n5889 = n5879;
      19'b0000000000000010000: n5889 = 1'b0;
      19'b0000000000000001000: n5889 = n5878;
      19'b0000000000000000100: n5889 = n5877;
      19'b0000000000000000010: n5889 = 1'b1;
      19'b0000000000000000001: n5889 = 1'b0;
      default: n5889 = n5887;
    endcase
  assign n5890 = n5493[8]; // extract
  assign n5891 = n5496[24]; // extract
  assign n5892 = n5501[24]; // extract
  assign n5893 = n5506[24]; // extract
  assign n5894 = n5513[24]; // extract
  assign n5895 = n5519[8]; // extract
  assign n5900 = xcsr_rdata_i[24]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5902 = 1'b0;
      19'b0100000000000000000: n5902 = 1'b0;
      19'b0010000000000000000: n5902 = 1'b0;
      19'b0001000000000000000: n5902 = 1'b0;
      19'b0000100000000000000: n5902 = 1'b0;
      19'b0000010000000000000: n5902 = 1'b0;
      19'b0000001000000000000: n5902 = 1'b1;
      19'b0000000100000000000: n5902 = 1'b0;
      19'b0000000010000000000: n5902 = 1'b0;
      19'b0000000001000000000: n5902 = n5895;
      19'b0000000000100000000: n5902 = n5894;
      19'b0000000000010000000: n5902 = 1'b0;
      19'b0000000000001000000: n5902 = n5893;
      19'b0000000000000100000: n5902 = n5892;
      19'b0000000000000010000: n5902 = 1'b0;
      19'b0000000000000001000: n5902 = n5891;
      19'b0000000000000000100: n5902 = n5890;
      19'b0000000000000000010: n5902 = 1'b0;
      19'b0000000000000000001: n5902 = 1'b0;
      default: n5902 = n5900;
    endcase
  assign n5903 = n5493[9]; // extract
  assign n5904 = n5496[25]; // extract
  assign n5905 = n5501[25]; // extract
  assign n5906 = n5506[25]; // extract
  assign n5907 = n5513[25]; // extract
  assign n5908 = n5519[9]; // extract
  assign n5913 = xcsr_rdata_i[25]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5915 = 1'b0;
      19'b0100000000000000000: n5915 = 1'b0;
      19'b0010000000000000000: n5915 = 1'b0;
      19'b0001000000000000000: n5915 = 1'b0;
      19'b0000100000000000000: n5915 = 1'b0;
      19'b0000010000000000000: n5915 = 1'b0;
      19'b0000001000000000000: n5915 = 1'b0;
      19'b0000000100000000000: n5915 = 1'b0;
      19'b0000000010000000000: n5915 = 1'b0;
      19'b0000000001000000000: n5915 = n5908;
      19'b0000000000100000000: n5915 = n5907;
      19'b0000000000010000000: n5915 = 1'b0;
      19'b0000000000001000000: n5915 = n5906;
      19'b0000000000000100000: n5915 = n5905;
      19'b0000000000000010000: n5915 = 1'b0;
      19'b0000000000000001000: n5915 = n5904;
      19'b0000000000000000100: n5915 = n5903;
      19'b0000000000000000010: n5915 = 1'b0;
      19'b0000000000000000001: n5915 = 1'b0;
      default: n5915 = n5913;
    endcase
  assign n5916 = n5493[10]; // extract
  assign n5917 = n5496[26]; // extract
  assign n5918 = n5501[26]; // extract
  assign n5919 = n5506[26]; // extract
  assign n5920 = n5513[26]; // extract
  assign n5921 = n5519[10]; // extract
  assign n5926 = xcsr_rdata_i[26]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5928 = 1'b0;
      19'b0100000000000000000: n5928 = 1'b0;
      19'b0010000000000000000: n5928 = 1'b0;
      19'b0001000000000000000: n5928 = 1'b0;
      19'b0000100000000000000: n5928 = 1'b0;
      19'b0000010000000000000: n5928 = 1'b0;
      19'b0000001000000000000: n5928 = 1'b0;
      19'b0000000100000000000: n5928 = 1'b0;
      19'b0000000010000000000: n5928 = 1'b0;
      19'b0000000001000000000: n5928 = n5921;
      19'b0000000000100000000: n5928 = n5920;
      19'b0000000000010000000: n5928 = 1'b0;
      19'b0000000000001000000: n5928 = n5919;
      19'b0000000000000100000: n5928 = n5918;
      19'b0000000000000010000: n5928 = 1'b0;
      19'b0000000000000001000: n5928 = n5917;
      19'b0000000000000000100: n5928 = n5916;
      19'b0000000000000000010: n5928 = 1'b0;
      19'b0000000000000000001: n5928 = 1'b0;
      default: n5928 = n5926;
    endcase
  assign n5929 = n5493[11]; // extract
  assign n5930 = n5496[27]; // extract
  assign n5931 = n5501[27]; // extract
  assign n5932 = n5506[27]; // extract
  assign n5933 = n5513[27]; // extract
  assign n5934 = n5519[11]; // extract
  assign n5939 = xcsr_rdata_i[27]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5941 = 1'b0;
      19'b0100000000000000000: n5941 = 1'b0;
      19'b0010000000000000000: n5941 = 1'b0;
      19'b0001000000000000000: n5941 = 1'b0;
      19'b0000100000000000000: n5941 = 1'b0;
      19'b0000010000000000000: n5941 = 1'b0;
      19'b0000001000000000000: n5941 = 1'b0;
      19'b0000000100000000000: n5941 = 1'b0;
      19'b0000000010000000000: n5941 = 1'b0;
      19'b0000000001000000000: n5941 = n5934;
      19'b0000000000100000000: n5941 = n5933;
      19'b0000000000010000000: n5941 = 1'b0;
      19'b0000000000001000000: n5941 = n5932;
      19'b0000000000000100000: n5941 = n5931;
      19'b0000000000000010000: n5941 = 1'b0;
      19'b0000000000000001000: n5941 = n5930;
      19'b0000000000000000100: n5941 = n5929;
      19'b0000000000000000010: n5941 = 1'b0;
      19'b0000000000000000001: n5941 = 1'b0;
      default: n5941 = n5939;
    endcase
  assign n5942 = n5493[12]; // extract
  assign n5943 = n5496[28]; // extract
  assign n5944 = n5501[28]; // extract
  assign n5945 = n5506[28]; // extract
  assign n5946 = n5513[28]; // extract
  assign n5947 = n5519[12]; // extract
  assign n5952 = xcsr_rdata_i[28]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5954 = 1'b0;
      19'b0100000000000000000: n5954 = 1'b1;
      19'b0010000000000000000: n5954 = 1'b0;
      19'b0001000000000000000: n5954 = 1'b0;
      19'b0000100000000000000: n5954 = 1'b0;
      19'b0000010000000000000: n5954 = 1'b0;
      19'b0000001000000000000: n5954 = 1'b0;
      19'b0000000100000000000: n5954 = 1'b0;
      19'b0000000010000000000: n5954 = 1'b0;
      19'b0000000001000000000: n5954 = n5947;
      19'b0000000000100000000: n5954 = n5946;
      19'b0000000000010000000: n5954 = 1'b0;
      19'b0000000000001000000: n5954 = n5945;
      19'b0000000000000100000: n5954 = n5944;
      19'b0000000000000010000: n5954 = 1'b0;
      19'b0000000000000001000: n5954 = n5943;
      19'b0000000000000000100: n5954 = n5942;
      19'b0000000000000000010: n5954 = 1'b0;
      19'b0000000000000000001: n5954 = 1'b0;
      default: n5954 = n5952;
    endcase
  assign n5955 = n5493[13]; // extract
  assign n5956 = n5496[29]; // extract
  assign n5957 = n5501[29]; // extract
  assign n5958 = n5506[29]; // extract
  assign n5959 = n5513[29]; // extract
  assign n5960 = n5519[13]; // extract
  assign n5965 = xcsr_rdata_i[29]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5967 = 1'b0;
      19'b0100000000000000000: n5967 = 1'b0;
      19'b0010000000000000000: n5967 = 1'b0;
      19'b0001000000000000000: n5967 = 1'b0;
      19'b0000100000000000000: n5967 = 1'b0;
      19'b0000010000000000000: n5967 = 1'b0;
      19'b0000001000000000000: n5967 = 1'b0;
      19'b0000000100000000000: n5967 = 1'b0;
      19'b0000000010000000000: n5967 = 1'b0;
      19'b0000000001000000000: n5967 = n5960;
      19'b0000000000100000000: n5967 = n5959;
      19'b0000000000010000000: n5967 = 1'b0;
      19'b0000000000001000000: n5967 = n5958;
      19'b0000000000000100000: n5967 = n5957;
      19'b0000000000000010000: n5967 = 1'b0;
      19'b0000000000000001000: n5967 = n5956;
      19'b0000000000000000100: n5967 = n5955;
      19'b0000000000000000010: n5967 = 1'b0;
      19'b0000000000000000001: n5967 = 1'b0;
      default: n5967 = n5965;
    endcase
  assign n5968 = n5487[0]; // extract
  assign n5969 = n5493[14]; // extract
  assign n5970 = n5496[30]; // extract
  assign n5971 = n5501[30]; // extract
  assign n5972 = n5506[30]; // extract
  assign n5973 = n5513[30]; // extract
  assign n5974 = n5519[14]; // extract
  assign n5979 = xcsr_rdata_i[30]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5981 = 1'b0;
      19'b0100000000000000000: n5981 = 1'b0;
      19'b0010000000000000000: n5981 = 1'b0;
      19'b0001000000000000000: n5981 = 1'b0;
      19'b0000100000000000000: n5981 = 1'b0;
      19'b0000010000000000000: n5981 = 1'b0;
      19'b0000001000000000000: n5981 = 1'b0;
      19'b0000000100000000000: n5981 = 1'b0;
      19'b0000000010000000000: n5981 = 1'b0;
      19'b0000000001000000000: n5981 = n5974;
      19'b0000000000100000000: n5981 = n5973;
      19'b0000000000010000000: n5981 = 1'b0;
      19'b0000000000001000000: n5981 = n5972;
      19'b0000000000000100000: n5981 = n5971;
      19'b0000000000000010000: n5981 = 1'b0;
      19'b0000000000000001000: n5981 = n5970;
      19'b0000000000000000100: n5981 = n5969;
      19'b0000000000000000010: n5981 = n5968;
      19'b0000000000000000001: n5981 = 1'b0;
      default: n5981 = n5979;
    endcase
  assign n5982 = n5487[1]; // extract
  assign n5983 = n5493[15]; // extract
  assign n5984 = n5496[31]; // extract
  assign n5985 = n5501[31]; // extract
  assign n5986 = n5506[31]; // extract
  assign n5987 = n5513[31]; // extract
  assign n5988 = n5519[15]; // extract
  assign n5993 = xcsr_rdata_i[31]; // extract
  /* ../../rtl/core/neorv32_cpu_control.vhd:1183:9  */
  always @*
    case (n5604)
      19'b1000000000000000000: n5995 = 1'b0;
      19'b0100000000000000000: n5995 = 1'b0;
      19'b0010000000000000000: n5995 = 1'b0;
      19'b0001000000000000000: n5995 = 1'b0;
      19'b0000100000000000000: n5995 = 1'b0;
      19'b0000010000000000000: n5995 = 1'b0;
      19'b0000001000000000000: n5995 = 1'b0;
      19'b0000000100000000000: n5995 = 1'b0;
      19'b0000000010000000000: n5995 = 1'b0;
      19'b0000000001000000000: n5995 = n5988;
      19'b0000000000100000000: n5995 = n5987;
      19'b0000000000010000000: n5995 = n5509;
      19'b0000000000001000000: n5995 = n5986;
      19'b0000000000000100000: n5995 = n5985;
      19'b0000000000000010000: n5995 = 1'b0;
      19'b0000000000000001000: n5995 = n5984;
      19'b0000000000000000100: n5995 = n5983;
      19'b0000000000000000010: n5995 = n5982;
      19'b0000000000000000001: n5995 = 1'b0;
      default: n5995 = n5993;
    endcase
  assign n5996 = {n5995, n5981, n5967, n5954, n5941, n5928, n5915, n5902, n5889, n5876, n5863, n5850, n5837, n5824, n5811, n5798, n5785, n5774, n5763, n5752, n5741, n5730, n5719, n5708, n5697, n5686, n5675, n5664, n5652, n5640, n5628, n5616};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1182:7  */
  assign n5998 = n5459 ? n5996 : 32'b00000000000000000000000000000000;
  /* ../../rtl/core/neorv32_cpu_control.vhd:206:5  */
  always @(posedge clk_i or posedge n2775)
    if (n2775)
      n6004 <= n2785;
    else
      n6004 <= exec_nxt;
  /* ../../rtl/core/neorv32_cpu_control.vhd:198:5  */
  assign n6005 = {n3420, n3418, n3416};
  /* ../../rtl/core/neorv32_cpu_control.vhd:206:5  */
  always @(posedge clk_i or posedge n2775)
    if (n2775)
      n6006 <= 264'b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;
    else
      n6006 <= ctrl_nxt;
  /* ../../rtl/core/neorv32_cpu_control.vhd:198:5  */
  assign n6007 = {n3454, n3467, n3452, n3451, n3449, n3466, n3447, n3446, n3445, n3443, n3441, n3439, n3437, n2921, n3436, n3435, n3434, n3432, n3430, n3460, n3428, n3426, n3457, n3424};
  /* ../../rtl/core/neorv32_cpu_control.vhd:842:5  */
  always @(posedge clk_i or posedge n4917)
    if (n4917)
      n6008 <= 1'b0;
    else
      n6008 <= n4940;
  /* ../../rtl/core/neorv32_cpu_control.vhd:804:5  */
  always @(posedge clk_i or posedge n4669)
    if (n4669)
      n6009 <= n4892;
    else
      n6009 <= n4887;
  /* ../../rtl/core/neorv32_cpu_control.vhd:804:5  */
  always @(posedge clk_i or posedge n4669)
    if (n4669)
      n6010 <= 12'b000000000000;
    else
      n6010 <= n4886;
  /* ../../rtl/core/neorv32_cpu_control.vhd:800:5  */
  assign n6011 = {n3473, n4666, n3471, n3470, n3469, n3468, n6008, n5148, n5050, n5047, n5045, n6009, n4977, n6010};
  /* ../../rtl/core/neorv32_cpu_control.vhd:1020:5  */
  always @(posedge clk_i or posedge n5196)
    if (n5196)
      n6012 <= n5453;
    else
      n6012 <= n5451;
  /* ../../rtl/core/neorv32_cpu_control.vhd:1180:5  */
  always @(posedge clk_i or posedge n5457)
    if (n5457)
      n6013 <= 32'b00000000000000000000000000000000;
    else
      n6013 <= n5998;
  /* ../../rtl/core/neorv32_cpu_control.vhd:996:5  */
  assign n6015 = {1'b0, 1'b0, 1'b0, 1'b0, 1'b0};
  /* ../../rtl/core/neorv32_cpu_control.vhd:776:5  */
  always @(posedge clk_i or posedge n4642)
    if (n4642)
      n6016 <= 10'b0000000000;
    else
      n6016 <= n4650;
  /* ../../rtl/core/neorv32_cpu_control.vhd:774:5  */
  assign n6017 = {n4419, n4437, n4449};
  /* ../../rtl/core/neorv32_cpu_control.vhd:800:5  */
  assign n6018 = {n3685, n3676, n3670, n3664, n3656, n3650, n3644, n3635, n3627, 1'b0, n3620};
  /* ../../rtl/core/neorv32_cpu_control.vhd:840:5  */
  assign n6019 = {n3615, n3614, n3613, n3612, n3611, n3610, n3609, n3608, n3607, cnt_event, csr_wdata, n3606, n3605, n3604, n3602, n3598, n3592, n3587, n3586, n3585, n3584, n3566, n3548, n3530, n3529, n3528, n3527, n3526, n3525, n3524, n3523, n3522, n3521, n3520, n3490, n3487, n3484, n3480, n3475};
endmodule

module neorv32_cpu_frontend_0_0e356ba505631fbf715758bed27d503f8b260e3a
  (input  clk_i,
   input  rstn_i,
   input  \ctrl_i_ctrl_i[if_reset] ,
   input  \ctrl_i_ctrl_i[if_ready] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_cur] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_nxt] ,
   input  [31:0] \ctrl_i_ctrl_i[pc_ret] ,
   input  \ctrl_i_ctrl_i[rf_wb_en] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs1] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rs2] ,
   input  [4:0] \ctrl_i_ctrl_i[rf_rd] ,
   input  \ctrl_i_ctrl_i[rf_zero] ,
   input  [2:0] \ctrl_i_ctrl_i[alu_op] ,
   input  \ctrl_i_ctrl_i[alu_sub] ,
   input  \ctrl_i_ctrl_i[alu_opa_mux] ,
   input  \ctrl_i_ctrl_i[alu_opb_mux] ,
   input  \ctrl_i_ctrl_i[alu_unsigned] ,
   input  [31:0] \ctrl_i_ctrl_i[alu_imm] ,
   input  \ctrl_i_ctrl_i[alu_cp_alu] ,
   input  \ctrl_i_ctrl_i[alu_cp_cfu] ,
   input  \ctrl_i_ctrl_i[alu_cp_fpu] ,
   input  \ctrl_i_ctrl_i[lsu_req] ,
   input  \ctrl_i_ctrl_i[lsu_rd] ,
   input  \ctrl_i_ctrl_i[lsu_wr] ,
   input  \ctrl_i_ctrl_i[lsu_mo_en] ,
   input  \ctrl_i_ctrl_i[lsu_mi_en] ,
   input  \ctrl_i_ctrl_i[lsu_priv] ,
   input  \ctrl_i_ctrl_i[csr_we] ,
   input  \ctrl_i_ctrl_i[csr_re] ,
   input  [11:0] \ctrl_i_ctrl_i[csr_addr] ,
   input  [31:0] \ctrl_i_ctrl_i[csr_wdata] ,
   input  [10:0] \ctrl_i_ctrl_i[cnt_event] ,
   input  [2:0] \ctrl_i_ctrl_i[ir_funct3] ,
   input  [11:0] \ctrl_i_ctrl_i[ir_funct12] ,
   input  [6:0] \ctrl_i_ctrl_i[ir_opcode] ,
   input  [15:0] \ctrl_i_ctrl_i[ir_rvc] ,
   input  \ctrl_i_ctrl_i[cpu_priv] ,
   input  \ctrl_i_ctrl_i[cpu_trap] ,
   input  \ctrl_i_ctrl_i[cpu_sync_exc] ,
   input  \ctrl_i_ctrl_i[cpu_debug] ,
   input  [1:0] \ctrl_i_ctrl_i[cpu_fence] ,
   input  \ibus_rsp_i_ibus_rsp_i[ack] ,
   input  \ibus_rsp_i_ibus_rsp_i[err] ,
   input  [31:0] \ibus_rsp_i_ibus_rsp_i[data] ,
   input  pmp_err_i,
   output [4:0] \ibus_req_o_ibus_req_o[meta] ,
   output [31:0] \ibus_req_o_ibus_req_o[addr] ,
   output [31:0] \ibus_req_o_ibus_req_o[data] ,
   output [3:0] \ibus_req_o_ibus_req_o[ben] ,
   output \ibus_req_o_ibus_req_o[stb] ,
   output \ibus_req_o_ibus_req_o[rw] ,
   output \ibus_req_o_ibus_req_o[amo] ,
   output [3:0] \ibus_req_o_ibus_req_o[amoop] ,
   output \ibus_req_o_ibus_req_o[burst] ,
   output \ibus_req_o_ibus_req_o[lock] ,
   output [31:0] pmp_addr_o,
   output pmp_priv_o,
   output \frontend_o_frontend_o[valid] ,
   output [31:0] \frontend_o_frontend_o[i32] ,
   output [15:0] \frontend_o_frontend_o[i16] ,
   output \frontend_o_frontend_o[compr] ,
   output \frontend_o_frontend_o[fault] );
  wire [263:0] n2442;
  wire [4:0] n2444;
  wire [31:0] n2445;
  wire [31:0] n2446;
  wire [3:0] n2447;
  wire n2448;
  wire n2449;
  wire n2450;
  wire [3:0] n2451;
  wire n2452;
  wire n2453;
  wire [33:0] n2454;
  wire n2458;
  wire [31:0] n2459;
  wire [15:0] n2460;
  wire n2461;
  wire n2462;
  wire [36:0] fetch;
  wire restart;
  wire [75:0] ipb;
  wire align_q;
  wire align_set;
  wire align_clr;
  wire [1:0] issue_valid;
  wire [15:0] cmd16;
  wire [31:0] cmd32;
  wire n2464;
  wire [1:0] n2471;
  wire [31:0] n2473;
  wire n2474;
  wire n2475;
  wire n2478;
  wire [1:0] n2479;
  wire n2481;
  wire [1:0] n2484;
  wire [1:0] n2485;
  wire [1:0] n2486;
  wire n2488;
  wire n2489;
  wire [31:0] n2490;
  wire [31:0] n2492;
  wire [29:0] n2494;
  wire n2495;
  wire [1:0] n2498;
  wire [31:0] n2499;
  wire [1:0] n2500;
  wire [1:0] n2501;
  wire [31:0] n2502;
  wire [31:0] n2503;
  wire n2505;
  wire [2:0] n2507;
  reg [1:0] n2508;
  wire n2509;
  reg n2510;
  wire [31:0] n2511;
  reg [31:0] n2512;
  wire n2513;
  reg n2514;
  wire n2515;
  reg n2516;
  wire [36:0] n2517;
  wire [36:0] n2519;
  wire n2522;
  wire n2523;
  wire n2524;
  wire [29:0] n2525;
  wire [31:0] n2527;
  wire n2528;
  wire n2529;
  wire [2:0] n2531;
  wire n2532;
  wire [3:0] n2533;
  wire [4:0] n2535;
  wire [29:0] n2536;
  wire [31:0] n2538;
  wire [1:0] n2540;
  wire n2542;
  wire [1:0] n2543;
  wire n2545;
  wire n2546;
  wire n2547;
  wire n2556;
  wire n2557;
  wire [15:0] n2558;
  wire [16:0] n2559;
  wire n2560;
  wire n2561;
  wire [15:0] n2562;
  wire [16:0] n2563;
  wire [1:0] n2565;
  wire n2567;
  wire n2568;
  wire n2569;
  wire n2570;
  wire n2571;
  wire n2573;
  wire n2574;
  wire n2575;
  wire [1:0] n2578;
  wire n2580;
  wire n2581;
  wire n2582;
  wire n2583;
  wire [16:0] n2585;
  wire n2586;
  wire prefetch_buffer_n1_ipb_inst_n2587;
  wire n2588;
  wire [16:0] prefetch_buffer_n1_ipb_inst_n2589;
  wire prefetch_buffer_n1_ipb_inst_n2590;
  wire [16:0] n2597;
  wire n2598;
  wire prefetch_buffer_n2_ipb_inst_n2599;
  wire n2600;
  wire [16:0] prefetch_buffer_n2_ipb_inst_n2601;
  wire prefetch_buffer_n2_ipb_inst_n2602;
  wire [15:0] n2610;
  wire n2611;
  wire [15:0] n2612;
  wire [15:0] n2613;
  wire n2615;
  wire n2617;
  wire n2618;
  wire n2619;
  wire n2620;
  wire n2621;
  wire n2622;
  wire n2623;
  wire n2624;
  wire n2625;
  wire n2626;
  wire n2632;
  wire [1:0] n2633;
  wire n2635;
  wire n2636;
  wire n2637;
  wire n2639;
  wire n2641;
  wire n2642;
  wire n2643;
  wire n2644;
  wire n2645;
  wire n2646;
  wire n2647;
  wire n2648;
  wire n2649;
  wire [15:0] n2650;
  wire [15:0] n2651;
  wire [31:0] n2652;
  wire [1:0] n2654;
  wire [1:0] n2655;
  wire [31:0] n2656;
  wire [1:0] n2657;
  wire n2659;
  wire [1:0] n2660;
  wire [1:0] n2661;
  wire [1:0] n2662;
  wire [1:0] n2663;
  wire n2665;
  wire n2666;
  wire n2668;
  wire n2669;
  wire n2671;
  wire n2672;
  wire n2673;
  wire n2674;
  wire n2675;
  wire n2676;
  wire n2677;
  wire n2678;
  wire n2679;
  wire [15:0] n2680;
  wire [15:0] n2681;
  wire [31:0] n2682;
  wire [1:0] n2684;
  wire [1:0] n2685;
  wire [31:0] n2686;
  wire [1:0] n2687;
  wire n2689;
  wire [1:0] n2690;
  wire [1:0] n2691;
  wire [1:0] n2692;
  wire [31:0] n2693;
  wire [1:0] n2694;
  wire n2696;
  wire n2699;
  wire [1:0] n2701;
  wire n2703;
  wire n2704;
  wire n2705;
  wire n2706;
  wire n2707;
  wire n2708;
  wire n2709;
  wire n2710;
  wire n2711;
  reg [36:0] n2712;
  wire [75:0] n2713;
  reg n2714;
  wire [81:0] n2715;
  wire [50:0] n2716;
  assign \ibus_req_o_ibus_req_o[meta]  = n2444; //(module output)
  assign \ibus_req_o_ibus_req_o[addr]  = n2445; //(module output)
  assign \ibus_req_o_ibus_req_o[data]  = n2446; //(module output)
  assign \ibus_req_o_ibus_req_o[ben]  = n2447; //(module output)
  assign \ibus_req_o_ibus_req_o[stb]  = n2448; //(module output)
  assign \ibus_req_o_ibus_req_o[rw]  = n2449; //(module output)
  assign \ibus_req_o_ibus_req_o[amo]  = n2450; //(module output)
  assign \ibus_req_o_ibus_req_o[amoop]  = n2451; //(module output)
  assign \ibus_req_o_ibus_req_o[burst]  = n2452; //(module output)
  assign \ibus_req_o_ibus_req_o[lock]  = n2453; //(module output)
  assign pmp_addr_o = n2527; //(module output)
  assign pmp_priv_o = n2528; //(module output)
  assign \frontend_o_frontend_o[valid]  = n2458; //(module output)
  assign \frontend_o_frontend_o[i32]  = n2459; //(module output)
  assign \frontend_o_frontend_o[i16]  = n2460; //(module output)
  assign \frontend_o_frontend_o[compr]  = n2461; //(module output)
  assign \frontend_o_frontend_o[fault]  = n2462; //(module output)
  /* ../../rtl/core/neorv32_sysinfo.vhd:167:35  */
  assign n2442 = {\ctrl_i_ctrl_i[cpu_fence] , \ctrl_i_ctrl_i[cpu_debug] , \ctrl_i_ctrl_i[cpu_sync_exc] , \ctrl_i_ctrl_i[cpu_trap] , \ctrl_i_ctrl_i[cpu_priv] , \ctrl_i_ctrl_i[ir_rvc] , \ctrl_i_ctrl_i[ir_opcode] , \ctrl_i_ctrl_i[ir_funct12] , \ctrl_i_ctrl_i[ir_funct3] , \ctrl_i_ctrl_i[cnt_event] , \ctrl_i_ctrl_i[csr_wdata] , \ctrl_i_ctrl_i[csr_addr] , \ctrl_i_ctrl_i[csr_re] , \ctrl_i_ctrl_i[csr_we] , \ctrl_i_ctrl_i[lsu_priv] , \ctrl_i_ctrl_i[lsu_mi_en] , \ctrl_i_ctrl_i[lsu_mo_en] , \ctrl_i_ctrl_i[lsu_wr] , \ctrl_i_ctrl_i[lsu_rd] , \ctrl_i_ctrl_i[lsu_req] , \ctrl_i_ctrl_i[alu_cp_fpu] , \ctrl_i_ctrl_i[alu_cp_cfu] , \ctrl_i_ctrl_i[alu_cp_alu] , \ctrl_i_ctrl_i[alu_imm] , \ctrl_i_ctrl_i[alu_unsigned] , \ctrl_i_ctrl_i[alu_opb_mux] , \ctrl_i_ctrl_i[alu_opa_mux] , \ctrl_i_ctrl_i[alu_sub] , \ctrl_i_ctrl_i[alu_op] , \ctrl_i_ctrl_i[rf_zero] , \ctrl_i_ctrl_i[rf_rd] , \ctrl_i_ctrl_i[rf_rs2] , \ctrl_i_ctrl_i[rf_rs1] , \ctrl_i_ctrl_i[rf_wb_en] , \ctrl_i_ctrl_i[pc_ret] , \ctrl_i_ctrl_i[pc_nxt] , \ctrl_i_ctrl_i[pc_cur] , \ctrl_i_ctrl_i[if_ready] , \ctrl_i_ctrl_i[if_reset] };
  assign n2444 = n2715[4:0]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:167:35  */
  assign n2445 = n2715[36:5]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:160:3  */
  assign n2446 = n2715[68:37]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n2447 = n2715[72:69]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:93:5  */
  assign n2448 = n2715[73]; // extract
  /* ../../rtl/core/neorv32_package.vhd:916:12  */
  assign n2449 = n2715[74]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1216:14  */
  assign n2450 = n2715[75]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:89:3  */
  assign n2451 = n2715[79:76]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:162:5  */
  assign n2452 = n2715[80]; // extract
  assign n2453 = n2715[81]; // extract
  assign n2454 = {\ibus_rsp_i_ibus_rsp_i[data] , \ibus_rsp_i_ibus_rsp_i[err] , \ibus_rsp_i_ibus_rsp_i[ack] };
  assign n2458 = n2716[0]; // extract
  assign n2459 = n2716[32:1]; // extract
  assign n2460 = n2716[48:33]; // extract
  assign n2461 = n2716[49]; // extract
  assign n2462 = n2716[50]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:75:10  */
  assign fetch = n2712; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:78:10  */
  assign restart = n2524; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:87:10  */
  assign ipb = n2713; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:90:10  */
  assign align_q = n2714; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:90:19  */
  assign align_set = n2696; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:90:30  */
  assign align_clr = n2699; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:91:10  */
  assign issue_valid = n2701; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:92:10  */
  assign cmd16 = n2612; // (signal)
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:105:16  */
  assign n2464 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:112:18  */
  assign n2471 = fetch[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:117:33  */
  assign n2473 = n2442[65:34]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:118:33  */
  assign n2474 = n2442[258]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:119:33  */
  assign n2475 = n2442[261]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:114:9  */
  assign n2478 = n2471 == 2'b00;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:125:19  */
  assign n2479 = ipb[73:72]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:125:24  */
  assign n2481 = n2479 == 2'b11;
  assign n2484 = fetch[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:127:11  */
  assign n2485 = restart ? 2'b00 : n2484;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:125:11  */
  assign n2486 = n2481 ? 2'b10 : n2485;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:122:9  */
  assign n2488 = n2471 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:134:26  */
  assign n2489 = n2454[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:135:63  */
  assign n2490 = fetch[34:3]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:135:69  */
  assign n2492 = n2490 + 32'b00000000000000000000000000000100;
  assign n2494 = n2492[31:2]; // extract
  assign n2495 = n2492[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:137:13  */
  assign n2498 = restart ? 2'b00 : 2'b01;
  assign n2499 = {n2494, 1'b0, n2495};
  assign n2500 = fetch[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:134:11  */
  assign n2501 = n2489 ? n2498 : n2500;
  assign n2502 = fetch[34:3]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:134:11  */
  assign n2503 = n2489 ? n2499 : n2502;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:131:9  */
  assign n2505 = n2471 == 2'b10;
  assign n2507 = {n2505, n2488, n2478};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:112:7  */
  always @*
    case (n2507)
      3'b100: n2508 = n2501;
      3'b010: n2508 = n2486;
      3'b001: n2508 = 2'b01;
      default: n2508 = 2'b00;
    endcase
  assign n2509 = fetch[2]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:112:7  */
  always @*
    case (n2507)
      3'b100: n2510 = restart;
      3'b010: n2510 = restart;
      3'b001: n2510 = 1'b0;
      default: n2510 = n2509;
    endcase
  assign n2511 = fetch[34:3]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:112:7  */
  always @*
    case (n2507)
      3'b100: n2512 = n2503;
      3'b010: n2512 = n2511;
      3'b001: n2512 = n2473;
      default: n2512 = n2511;
    endcase
  assign n2513 = fetch[35]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:112:7  */
  always @*
    case (n2507)
      3'b100: n2514 = n2513;
      3'b010: n2514 = n2513;
      3'b001: n2514 = n2474;
      default: n2514 = n2513;
    endcase
  assign n2515 = fetch[36]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:112:7  */
  always @*
    case (n2507)
      3'b100: n2516 = n2515;
      3'b010: n2516 = n2515;
      3'b001: n2516 = n2475;
      default: n2516 = n2515;
    endcase
  assign n2517 = {n2516, n2514, n2512, n2510, n2508};
  assign n2519 = {1'b0, 1'b1, 32'b00000000000000000000000000000000, 1'b1, 2'b00};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:153:20  */
  assign n2522 = fetch[2]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:153:36  */
  assign n2523 = n2442[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:153:26  */
  assign n2524 = n2522 | n2523;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:156:27  */
  assign n2525 = fetch[34:5]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:156:41  */
  assign n2527 = {n2525, 2'b00};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:157:23  */
  assign n2528 = fetch[35]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:160:74  */
  assign n2529 = fetch[36]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:160:66  */
  assign n2531 = {2'b00, n2529};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:160:88  */
  assign n2532 = fetch[35]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:160:80  */
  assign n2533 = {n2531, n2532};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:160:93  */
  assign n2535 = {n2533, 1'b1};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:161:33  */
  assign n2536 = fetch[34:5]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:161:47  */
  assign n2538 = {n2536, 2'b00};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:162:39  */
  assign n2540 = fetch[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:162:45  */
  assign n2542 = n2540 == 2'b01;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:162:67  */
  assign n2543 = ipb[73:72]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:162:72  */
  assign n2545 = n2543 == 2'b11;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:162:58  */
  assign n2546 = n2545 & n2542;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:162:27  */
  assign n2547 = n2546 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:172:31  */
  assign n2556 = n2454[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:172:35  */
  assign n2557 = n2556 | pmp_err_i;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:172:66  */
  assign n2558 = n2454[17:2]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:172:49  */
  assign n2559 = {n2557, n2558};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:173:31  */
  assign n2560 = n2454[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:173:35  */
  assign n2561 = n2560 | pmp_err_i;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:173:66  */
  assign n2562 = n2454[33:18]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:173:49  */
  assign n2563 = {n2561, n2562};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:176:32  */
  assign n2565 = fetch[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:176:38  */
  assign n2567 = n2565 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:176:67  */
  assign n2568 = n2454[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:176:51  */
  assign n2569 = n2568 & n2567;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:176:94  */
  assign n2570 = fetch[4]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:176:98  */
  assign n2571 = ~n2570;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:176:105  */
  assign n2573 = n2571 | 1'b0;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:176:78  */
  assign n2574 = n2573 & n2569;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:176:20  */
  assign n2575 = n2574 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:177:32  */
  assign n2578 = fetch[1:0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:177:38  */
  assign n2580 = n2578 == 2'b10;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:177:67  */
  assign n2581 = n2454[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:177:51  */
  assign n2582 = n2581 & n2580;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:177:20  */
  assign n2583 = n2582 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:194:27  */
  assign n2585 = ipb[33:17]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:195:24  */
  assign n2586 = ipb[68]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:198:24  */
  assign n2588 = ipb[70]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:183:5  */
  neorv32_cpu_frontend_ipb_1_17 prefetch_buffer_n1_ipb_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .clear_i(restart),
    .wdata_i(n2585),
    .we_i(n2586),
    .re_i(n2588),
    .free_o(prefetch_buffer_n1_ipb_inst_n2587),
    .rdata_o(prefetch_buffer_n1_ipb_inst_n2589),
    .avail_o(prefetch_buffer_n1_ipb_inst_n2590));
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:194:27  */
  assign n2597 = ipb[16:0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:195:24  */
  assign n2598 = ipb[69]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:198:24  */
  assign n2600 = ipb[71]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:183:5  */
  neorv32_cpu_frontend_ipb_1_17 prefetch_buffer_n2_ipb_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .clear_i(restart),
    .wdata_i(n2597),
    .we_i(n2598),
    .re_i(n2600),
    .free_o(prefetch_buffer_n2_ipb_inst_n2599),
    .rdata_o(prefetch_buffer_n2_ipb_inst_n2601),
    .avail_o(prefetch_buffer_n2_ipb_inst_n2602));
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:213:5  */
  neorv32_cpu_decompressor_5ba93c9db0cff93f52b521d7420e43f6eda2784f issue_enabled_neorv32_cpu_decompressor_inst (
    .instr_i(cmd16),
    .instr_o(cmd32));
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:223:26  */
  assign n2610 = ipb[66:51]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:223:54  */
  assign n2611 = ~align_q;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:223:40  */
  assign n2612 = n2611 ? n2610 : n2613;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:223:78  */
  assign n2613 = ipb[49:34]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:230:18  */
  assign n2615 = ~rstn_i;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:233:19  */
  assign n2617 = fetch[2]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:234:35  */
  assign n2618 = n2442[35]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:235:22  */
  assign n2619 = ipb[70]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:235:43  */
  assign n2620 = ipb[71]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:235:33  */
  assign n2621 = n2619 | n2620;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:236:36  */
  assign n2622 = ~align_clr;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:236:31  */
  assign n2623 = align_q & n2622;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:236:52  */
  assign n2624 = n2623 | align_set;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:235:9  */
  assign n2625 = n2621 ? n2624 : align_q;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:233:9  */
  assign n2626 = n2617 ? n2618 : n2625;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:247:19  */
  assign n2632 = ~align_q;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:248:25  */
  assign n2633 = ipb[52:51]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:248:38  */
  assign n2635 = n2633 != 2'b11;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:249:40  */
  assign n2636 = ipb[74]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:250:40  */
  assign n2637 = ipb[74]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:252:43  */
  assign n2639 = ipb[67]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:256:40  */
  assign n2641 = ipb[75]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:256:57  */
  assign n2642 = ipb[74]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:256:44  */
  assign n2643 = n2641 & n2642;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:257:40  */
  assign n2644 = ipb[75]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:257:57  */
  assign n2645 = ipb[74]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:257:44  */
  assign n2646 = n2644 & n2645;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:258:43  */
  assign n2647 = ipb[50]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:258:63  */
  assign n2648 = ipb[67]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:258:48  */
  assign n2649 = n2647 | n2648;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:259:43  */
  assign n2650 = ipb[49:34]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:259:71  */
  assign n2651 = ipb[66:51]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:259:57  */
  assign n2652 = {n2650, n2651};
  assign n2654 = {n2649, 1'b0};
  assign n2655 = {n2639, 1'b1};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:248:9  */
  assign n2656 = n2635 ? cmd32 : n2652;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:248:9  */
  assign n2657 = n2635 ? n2655 : n2654;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:248:9  */
  assign n2659 = n2635 ? n2636 : 1'b0;
  assign n2660 = {n2646, n2643};
  assign n2661 = {1'b0, n2637};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:248:9  */
  assign n2662 = n2635 ? n2661 : n2660;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:264:25  */
  assign n2663 = ipb[35:34]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:264:38  */
  assign n2665 = n2663 != 2'b11;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:265:40  */
  assign n2666 = ipb[75]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:267:40  */
  assign n2668 = ipb[75]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:268:43  */
  assign n2669 = ipb[50]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:272:40  */
  assign n2671 = ipb[74]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:272:57  */
  assign n2672 = ipb[75]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:272:44  */
  assign n2673 = n2671 & n2672;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:273:40  */
  assign n2674 = ipb[74]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:273:57  */
  assign n2675 = ipb[75]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:273:44  */
  assign n2676 = n2674 & n2675;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:274:43  */
  assign n2677 = ipb[67]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:274:63  */
  assign n2678 = ipb[50]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:274:48  */
  assign n2679 = n2677 | n2678;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:275:43  */
  assign n2680 = ipb[66:51]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:275:71  */
  assign n2681 = ipb[49:34]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:275:57  */
  assign n2682 = {n2680, n2681};
  assign n2684 = {n2679, 1'b0};
  assign n2685 = {n2669, 1'b1};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:264:9  */
  assign n2686 = n2665 ? cmd32 : n2682;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:264:9  */
  assign n2687 = n2665 ? n2685 : n2684;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:264:9  */
  assign n2689 = n2665 ? n2666 : 1'b0;
  assign n2690 = {n2676, n2673};
  assign n2691 = {n2668, 1'b0};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:264:9  */
  assign n2692 = n2665 ? n2691 : n2690;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:247:7  */
  assign n2693 = n2632 ? n2656 : n2686;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:247:7  */
  assign n2694 = n2632 ? n2657 : n2687;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:247:7  */
  assign n2696 = n2632 ? n2659 : 1'b0;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:247:7  */
  assign n2699 = n2632 ? 1'b0 : n2689;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:247:7  */
  assign n2701 = n2632 ? n2662 : n2692;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:284:36  */
  assign n2703 = issue_valid[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:284:54  */
  assign n2704 = issue_valid[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:284:40  */
  assign n2705 = n2703 | n2704;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:287:29  */
  assign n2706 = issue_valid[0]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:287:44  */
  assign n2707 = n2442[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:287:33  */
  assign n2708 = n2706 & n2707;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:288:29  */
  assign n2709 = issue_valid[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:288:44  */
  assign n2710 = n2442[1]; // extract
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:288:33  */
  assign n2711 = n2709 & n2710;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:111:5  */
  always @(posedge clk_i or posedge n2464)
    if (n2464)
      n2712 <= n2519;
    else
      n2712 <= n2517;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:105:5  */
  assign n2713 = {prefetch_buffer_n2_ipb_inst_n2602, prefetch_buffer_n1_ipb_inst_n2590, prefetch_buffer_n2_ipb_inst_n2599, prefetch_buffer_n1_ipb_inst_n2587, n2711, n2708, n2583, n2575, prefetch_buffer_n1_ipb_inst_n2589, prefetch_buffer_n2_ipb_inst_n2601, n2559, n2563};
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:232:7  */
  always @(posedge clk_i or posedge n2615)
    if (n2615)
      n2714 <= 1'b0;
    else
      n2714 <= n2626;
  /* ../../rtl/core/neorv32_cpu_frontend.vhd:230:7  */
  assign n2715 = {1'b0, 1'b0, 4'b0000, 1'b0, 1'b0, n2547, 4'b1111, 32'b00000000000000000000000000000000, n2538, n2535};
  assign n2716 = {n2694, cmd16, n2693, n2705};
endmodule

module neorv32_sysinfo_16_0_1_100000000_1_16384_8192_4_4_64_8880fc9ba9bd7b048f24af91b57aaee3b939071a
  (input  clk_i,
   input  rstn_i,
   input  [4:0] \bus_req_i_bus_req_i[meta] ,
   input  [31:0] \bus_req_i_bus_req_i[addr] ,
   input  [31:0] \bus_req_i_bus_req_i[data] ,
   input  [3:0] \bus_req_i_bus_req_i[ben] ,
   input  \bus_req_i_bus_req_i[stb] ,
   input  \bus_req_i_bus_req_i[rw] ,
   input  \bus_req_i_bus_req_i[amo] ,
   input  [3:0] \bus_req_i_bus_req_i[amoop] ,
   input  \bus_req_i_bus_req_i[burst] ,
   input  \bus_req_i_bus_req_i[lock] ,
   output \bus_rsp_o_bus_rsp_o[ack] ,
   output \bus_rsp_o_bus_rsp_o[err] ,
   output [31:0] \bus_rsp_o_bus_rsp_o[data] );
  wire [81:0] n2234;
  wire n2236;
  wire n2237;
  wire [31:0] n2238;
  wire [127:0] sysinfo;
  wire n2240;
  wire n2243;
  wire n2244;
  wire n2245;
  wire [1:0] n2246;
  wire n2248;
  wire n2249;
  wire [31:0] n2250;
  wire [7:0] n2259;
  wire [7:0] n2263;
  wire n2271;
  wire n2275;
  wire n2278;
  wire n2281;
  wire n2285;
  wire n2289;
  wire n2293;
  wire n2301;
  wire n2305;
  wire n2309;
  wire n2313;
  wire n2317;
  wire n2321;
  wire n2325;
  wire n2329;
  wire n2333;
  wire n2337;
  wire n2341;
  wire n2345;
  wire n2349;
  wire n2353;
  wire n2357;
  wire n2361;
  wire n2365;
  wire n2369;
  wire n2373;
  wire n2377;
  wire n2381;
  wire [3:0] n2385;
  wire [3:0] n2389;
  wire [3:0] n2393;
  wire [3:0] n2397;
  wire n2401;
  wire n2406;
  wire n2410;
  wire n2412;
  wire [1:0] n2413;
  wire [1:0] n2416;
  wire n2420;
  wire [1:0] n2421;
  wire n2423;
  wire n2424;
  wire n2427;
  wire [33:0] n2428;
  wire [33:0] n2430;
  wire [31:0] n2436;
  wire [31:0] n2437;
  reg [31:0] n2438;
  wire [127:0] n2439;
  reg [33:0] n2440;
  wire [31:0] n2441;
  assign \bus_rsp_o_bus_rsp_o[ack]  = n2236; //(module output)
  assign \bus_rsp_o_bus_rsp_o[err]  = n2237; //(module output)
  assign \bus_rsp_o_bus_rsp_o[data]  = n2238; //(module output)
  assign n2234 = {\bus_req_i_bus_req_i[lock] , \bus_req_i_bus_req_i[burst] , \bus_req_i_bus_req_i[amoop] , \bus_req_i_bus_req_i[amo] , \bus_req_i_bus_req_i[rw] , \bus_req_i_bus_req_i[stb] , \bus_req_i_bus_req_i[ben] , \bus_req_i_bus_req_i[data] , \bus_req_i_bus_req_i[addr] , \bus_req_i_bus_req_i[meta] };
  assign n2236 = n2440[0]; // extract
  assign n2237 = n2440[1]; // extract
  assign n2238 = n2440[33:2]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:83:10  */
  assign sysinfo = n2439; // (signal)
  /* ../../rtl/core/neorv32_sysinfo.vhd:91:16  */
  assign n2240 = ~rstn_i;
  /* ../../rtl/core/neorv32_sysinfo.vhd:94:21  */
  assign n2243 = n2234[73]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:94:47  */
  assign n2244 = n2234[74]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:94:32  */
  assign n2245 = n2244 & n2243;
  /* ../../rtl/core/neorv32_sysinfo.vhd:94:76  */
  assign n2246 = n2234[8:7]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:94:89  */
  assign n2248 = n2246 == 2'b00;
  /* ../../rtl/core/neorv32_sysinfo.vhd:94:57  */
  assign n2249 = n2248 & n2245;
  /* ../../rtl/core/neorv32_sysinfo.vhd:95:33  */
  assign n2250 = n2234[68:37]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:102:83  */
  assign n2259 = 1'b0 ? 8'b00001110 : 8'b00000000;
  /* ../../rtl/core/neorv32_sysinfo.vhd:103:83  */
  assign n2263 = 1'b0 ? 8'b00001101 : 8'b00000000;
  /* ../../rtl/core/neorv32_sysinfo.vhd:111:25  */
  assign n2271 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:112:25  */
  assign n2275 = 1'b1 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:113:25  */
  assign n2278 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:114:25  */
  assign n2281 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:115:25  */
  assign n2285 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:116:25  */
  assign n2289 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:117:25  */
  assign n2293 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:122:25  */
  assign n2301 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:123:25  */
  assign n2305 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:124:25  */
  assign n2309 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:125:25  */
  assign n2313 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:126:25  */
  assign n2317 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:127:25  */
  assign n2321 = 1'b1 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:128:25  */
  assign n2325 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:129:25  */
  assign n2329 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:130:25  */
  assign n2333 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:131:25  */
  assign n2337 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:132:25  */
  assign n2341 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:133:25  */
  assign n2345 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:134:25  */
  assign n2349 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:135:25  */
  assign n2353 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:136:25  */
  assign n2357 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:137:25  */
  assign n2361 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:138:25  */
  assign n2365 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:139:25  */
  assign n2369 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:140:25  */
  assign n2373 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:141:25  */
  assign n2377 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:142:25  */
  assign n2381 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:146:81  */
  assign n2385 = 1'b0 ? 4'b0110 : 4'b0000;
  /* ../../rtl/core/neorv32_sysinfo.vhd:147:81  */
  assign n2389 = 1'b0 ? 4'b0010 : 4'b0000;
  /* ../../rtl/core/neorv32_sysinfo.vhd:149:81  */
  assign n2393 = 1'b0 ? 4'b0110 : 4'b0000;
  /* ../../rtl/core/neorv32_sysinfo.vhd:150:81  */
  assign n2397 = 1'b0 ? 4'b0010 : 4'b0000;
  /* ../../rtl/core/neorv32_sysinfo.vhd:152:25  */
  assign n2401 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:155:25  */
  assign n2406 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_sysinfo.vhd:162:16  */
  assign n2410 = ~rstn_i;
  /* ../../rtl/core/neorv32_sysinfo.vhd:166:21  */
  assign n2412 = n2234[73]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:167:69  */
  assign n2413 = n2234[8:7]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:167:35  */
  assign n2416 = 2'b11 - n2413;
  /* ../../rtl/core/neorv32_sysinfo.vhd:169:23  */
  assign n2420 = n2234[74]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:169:52  */
  assign n2421 = n2234[8:7]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:169:65  */
  assign n2423 = n2421 != 2'b00;
  /* ../../rtl/core/neorv32_sysinfo.vhd:169:33  */
  assign n2424 = n2423 & n2420;
  /* ../../rtl/core/neorv32_sysinfo.vhd:169:9  */
  assign n2427 = n2424 ? 1'b1 : 1'b0;
  assign n2428 = {n2441, n2427, 1'b1};
  /* ../../rtl/core/neorv32_sysinfo.vhd:166:7  */
  assign n2430 = n2412 ? n2428 : 34'b0000000000000000000000000000000000;
  /* ../../rtl/core/neorv32_sysinfo.vhd:93:5  */
  assign n2436 = sysinfo[127:96]; // extract
  /* ../../rtl/core/neorv32_sysinfo.vhd:93:5  */
  assign n2437 = n2249 ? n2250 : n2436;
  /* ../../rtl/core/neorv32_sysinfo.vhd:93:5  */
  always @(posedge clk_i or posedge n2240)
    if (n2240)
      n2438 <= 32'b00000101111101011110000100000000;
    else
      n2438 <= n2437;
  /* ../../rtl/core/neorv32_sysinfo.vhd:91:5  */
  assign n2439 = {n2438, 5'b00000, 5'b00100, 2'b01, 4'b0001, n2263, n2259, n2381, n2377, n2373, n2369, n2365, n2361, n2357, n2353, n2349, n2345, n2341, n2337, n2333, n2329, n2325, n2321, n2317, n2313, n2309, n2305, n2301, 1'b0, 1'b0, 1'b0, 1'b0, n2293, n2289, n2285, n2281, n2278, n2275, n2271, 7'b0000000, n2406, 7'b0000000, n2401, n2397, n2393, n2389, n2385};
  /* ../../rtl/core/neorv32_sysinfo.vhd:164:5  */
  always @(posedge clk_i or posedge n2410)
    if (n2410)
      n2440 <= 34'b0000000000000000000000000000000000;
    else
      n2440 <= n2430;
  /* ../../rtl/core/neorv32_sysinfo.vhd:167:35  */
  assign n2441 = sysinfo[n2416 * 32 +: 32]; //(Bmux)
endmodule

module neorv32_clint_1
  (input  clk_i,
   input  rstn_i,
   input  [4:0] \bus_req_i_bus_req_i[meta] ,
   input  [31:0] \bus_req_i_bus_req_i[addr] ,
   input  [31:0] \bus_req_i_bus_req_i[data] ,
   input  [3:0] \bus_req_i_bus_req_i[ben] ,
   input  \bus_req_i_bus_req_i[stb] ,
   input  \bus_req_i_bus_req_i[rw] ,
   input  \bus_req_i_bus_req_i[amo] ,
   input  [3:0] \bus_req_i_bus_req_i[amoop] ,
   input  \bus_req_i_bus_req_i[burst] ,
   input  \bus_req_i_bus_req_i[lock] ,
   output \bus_rsp_o_bus_rsp_o[ack] ,
   output \bus_rsp_o_bus_rsp_o[err] ,
   output [31:0] \bus_rsp_o_bus_rsp_o[data] ,
   output [63:0] time_o,
   output mti_o,
   output msi_o);
  wire [81:0] n2091;
  wire n2093;
  wire n2094;
  wire [31:0] n2095;
  wire mtime_en;
  wire [1:0] mtime_we;
  wire [1:0] mtimecmp_we;
  wire [1:0] mtimecmp_re;
  wire mtimecmp_en;
  wire mswi_en;
  wire mswi;
  wire [31:0] mtimecmp_rd;
  wire [31:0] mswi_rd;
  wire [63:0] mtime;
  wire [31:0] mtime_rd;
  wire [31:0] rdata;
  localparam n2099 = 1'b1;
  wire [31:0] n2100;
  localparam n2101 = 1'b1;
  wire n2104;
  wire [12:0] n2105;
  wire n2107;
  wire n2108;
  wire n2109;
  wire n2111;
  wire n2112;
  wire n2113;
  wire n2114;
  wire n2115;
  wire n2116;
  wire n2117;
  wire n2118;
  wire n2119;
  wire n2121;
  wire [31:0] n2122;
  wire [31:0] n2123;
  wire n2124;
  wire [31:0] n2125;
  wire [31:0] n2126;
  wire [31:0] n2127;
  wire [31:0] neorv32_clint_mtimecmp_gen_n1_neorv32_clint_mtimecmp_inst_n2128;
  wire neorv32_clint_mtimecmp_gen_n1_neorv32_clint_mtimecmp_inst_n2129;
  wire n2135;
  wire [12:0] n2136;
  wire n2138;
  wire n2139;
  wire n2140;
  wire n2142;
  wire n2143;
  wire n2144;
  wire n2145;
  wire n2146;
  wire n2147;
  wire n2148;
  wire n2149;
  wire n2150;
  wire n2151;
  wire n2152;
  wire n2153;
  wire n2154;
  wire n2155;
  wire n2156;
  wire n2157;
  wire n2158;
  wire n2159;
  wire n2160;
  wire n2161;
  wire n2163;
  wire n2165;
  wire n2166;
  wire n2167;
  wire n2174;
  wire [13:0] n2175;
  wire n2177;
  wire n2178;
  wire n2179;
  wire n2182;
  wire [31:0] n2183;
  wire [31:0] n2185;
  wire [31:0] n2189;
  wire [31:0] n2190;
  wire [31:0] n2192;
  wire n2195;
  wire n2204;
  wire n2206;
  wire n2214;
  wire n2216;
  localparam [33:0] n2217 = 34'b0000000000000000000000000000000000;
  wire n2219;
  wire [31:0] n2220;
  wire [31:0] n2221;
  wire n2222;
  wire [33:0] n2223;
  wire [1:0] n2228;
  wire [1:0] n2229;
  wire [1:0] n2230;
  wire n2231;
  reg n2232;
  reg [33:0] n2233;
  assign \bus_rsp_o_bus_rsp_o[ack]  = n2093; //(module output)
  assign \bus_rsp_o_bus_rsp_o[err]  = n2094; //(module output)
  assign \bus_rsp_o_bus_rsp_o[data]  = n2095; //(module output)
  assign time_o = mtime; //(module output)
  assign mti_o = neorv32_clint_mtimecmp_gen_n1_neorv32_clint_mtimecmp_inst_n2129; //(module output)
  assign msi_o = mswi; //(module output)
  assign n2091 = {\bus_req_i_bus_req_i[lock] , \bus_req_i_bus_req_i[burst] , \bus_req_i_bus_req_i[amoop] , \bus_req_i_bus_req_i[amo] , \bus_req_i_bus_req_i[rw] , \bus_req_i_bus_req_i[stb] , \bus_req_i_bus_req_i[ben] , \bus_req_i_bus_req_i[data] , \bus_req_i_bus_req_i[addr] , \bus_req_i_bus_req_i[meta] };
  assign n2093 = n2233[0]; // extract
  assign n2094 = n2233[1]; // extract
  assign n2095 = n2233[33:2]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:58:10  */
  assign mtime_en = n2109; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:59:10  */
  assign mtime_we = n2228; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:63:10  */
  assign mtimecmp_we = n2229; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:64:10  */
  assign mtimecmp_re = n2230; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:65:10  */
  assign mtimecmp_en = n2140; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:68:10  */
  assign mswi_en = n2179; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:68:19  */
  assign mswi = n2232; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:72:10  */
  assign mtimecmp_rd = neorv32_clint_mtimecmp_gen_n1_neorv32_clint_mtimecmp_inst_n2128; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:73:10  */
  assign mswi_rd = n2183; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:75:10  */
  assign mtime_rd = n2122; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:76:10  */
  assign rdata = n2192; // (signal)
  /* ../../rtl/core/neorv32_clint.vhd:82:3  */
  neorv32_prim_cnt_64 neorv32_clint_mtime_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .inc_i(n2099),
    .we_i(mtime_we),
    .data_i(n2100),
    .oe_i(n2101),
    .cnt_o(mtime));
  /* ../../rtl/core/neorv32_clint.vhd:91:25  */
  assign n2100 = n2091[68:37]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:97:35  */
  assign n2104 = n2091[73]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:97:74  */
  assign n2105 = n2091[20:8]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:97:89  */
  assign n2107 = n2105 == 13'b1011111111111;
  /* ../../rtl/core/neorv32_clint.vhd:97:46  */
  assign n2108 = n2107 & n2104;
  /* ../../rtl/core/neorv32_clint.vhd:97:19  */
  assign n2109 = n2108 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_clint.vhd:98:41  */
  assign n2111 = n2091[74]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:98:27  */
  assign n2112 = mtime_en & n2111;
  /* ../../rtl/core/neorv32_clint.vhd:98:67  */
  assign n2113 = n2091[7]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:98:49  */
  assign n2114 = ~n2113;
  /* ../../rtl/core/neorv32_clint.vhd:98:44  */
  assign n2115 = n2112 & n2114;
  /* ../../rtl/core/neorv32_clint.vhd:99:41  */
  assign n2116 = n2091[74]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:99:27  */
  assign n2117 = mtime_en & n2116;
  /* ../../rtl/core/neorv32_clint.vhd:99:67  */
  assign n2118 = n2091[7]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:99:44  */
  assign n2119 = n2117 & n2118;
  /* ../../rtl/core/neorv32_clint.vhd:102:46  */
  assign n2121 = ~mtime_en;
  /* ../../rtl/core/neorv32_clint.vhd:102:31  */
  assign n2122 = n2121 ? 32'b00000000000000000000000000000000 : n2125;
  /* ../../rtl/core/neorv32_clint.vhd:103:20  */
  assign n2123 = mtime[63:32]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:103:55  */
  assign n2124 = n2091[7]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:102:53  */
  assign n2125 = n2124 ? n2123 : n2126;
  /* ../../rtl/core/neorv32_clint.vhd:103:76  */
  assign n2126 = mtime[31:0]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:121:28  */
  assign n2127 = n2091[68:37]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:114:5  */
  neorv32_clint_mtimecmp neorv32_clint_mtimecmp_gen_n1_neorv32_clint_mtimecmp_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .mtime_i(mtime),
    .we_i(mtimecmp_we),
    .re_i(mtimecmp_re),
    .wdata_i(n2127),
    .rdata_o(neorv32_clint_mtimecmp_gen_n1_neorv32_clint_mtimecmp_inst_n2128),
    .mti_o(neorv32_clint_mtimecmp_gen_n1_neorv32_clint_mtimecmp_inst_n2129));
  /* ../../rtl/core/neorv32_clint.vhd:127:43  */
  assign n2135 = n2091[73]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:127:82  */
  assign n2136 = n2091[20:8]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:127:97  */
  assign n2138 = n2136 == 13'b0100000000000;
  /* ../../rtl/core/neorv32_clint.vhd:127:54  */
  assign n2139 = n2138 & n2135;
  /* ../../rtl/core/neorv32_clint.vhd:127:27  */
  assign n2140 = n2139 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_clint.vhd:128:60  */
  assign n2142 = n2091[74]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:128:41  */
  assign n2143 = mtimecmp_en & n2142;
  /* ../../rtl/core/neorv32_clint.vhd:128:87  */
  assign n2144 = n2091[7]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:128:69  */
  assign n2145 = ~n2144;
  /* ../../rtl/core/neorv32_clint.vhd:128:64  */
  assign n2146 = n2143 & n2145;
  /* ../../rtl/core/neorv32_clint.vhd:129:60  */
  assign n2147 = n2091[74]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:129:41  */
  assign n2148 = mtimecmp_en & n2147;
  /* ../../rtl/core/neorv32_clint.vhd:129:87  */
  assign n2149 = n2091[7]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:129:64  */
  assign n2150 = n2148 & n2149;
  /* ../../rtl/core/neorv32_clint.vhd:130:60  */
  assign n2151 = n2091[74]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:130:46  */
  assign n2152 = ~n2151;
  /* ../../rtl/core/neorv32_clint.vhd:130:41  */
  assign n2153 = mtimecmp_en & n2152;
  /* ../../rtl/core/neorv32_clint.vhd:130:87  */
  assign n2154 = n2091[7]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:130:69  */
  assign n2155 = ~n2154;
  /* ../../rtl/core/neorv32_clint.vhd:130:64  */
  assign n2156 = n2153 & n2155;
  /* ../../rtl/core/neorv32_clint.vhd:131:60  */
  assign n2157 = n2091[74]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:131:46  */
  assign n2158 = ~n2157;
  /* ../../rtl/core/neorv32_clint.vhd:131:41  */
  assign n2159 = mtimecmp_en & n2158;
  /* ../../rtl/core/neorv32_clint.vhd:131:87  */
  assign n2160 = n2091[7]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:131:64  */
  assign n2161 = n2159 & n2160;
  /* ../../rtl/core/neorv32_clint.vhd:143:18  */
  assign n2163 = ~rstn_i;
  /* ../../rtl/core/neorv32_clint.vhd:146:46  */
  assign n2165 = n2091[74]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:146:31  */
  assign n2166 = n2165 & mswi_en;
  /* ../../rtl/core/neorv32_clint.vhd:147:36  */
  assign n2167 = n2091[37]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:156:39  */
  assign n2174 = n2091[73]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:156:78  */
  assign n2175 = n2091[20:7]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:156:93  */
  assign n2177 = n2175 == 14'b00000000000000;
  /* ../../rtl/core/neorv32_clint.vhd:156:50  */
  assign n2178 = n2177 & n2174;
  /* ../../rtl/core/neorv32_clint.vhd:156:23  */
  assign n2179 = n2178 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_clint.vhd:159:52  */
  assign n2182 = ~mswi_en;
  /* ../../rtl/core/neorv32_clint.vhd:159:35  */
  assign n2183 = n2182 ? 32'b00000000000000000000000000000000 : n2185;
  /* ../../rtl/core/neorv32_clint.vhd:159:84  */
  assign n2185 = {31'b0000000000000000000000000000000, mswi};
  /* ../../rtl/core/neorv32_clint.vhd:171:22  */
  assign n2189 = 32'b00000000000000000000000000000000 | mtimecmp_rd;
  /* ../../rtl/core/neorv32_clint.vhd:171:40  */
  assign n2190 = n2189 | mswi_rd;
  /* ../../rtl/core/neorv32_clint.vhd:173:23  */
  assign n2192 = mtime_rd | n2190;
  /* ../../rtl/core/neorv32_clint.vhd:181:16  */
  assign n2195 = ~rstn_i;
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n2204 = 1'b0 | mtimecmp_en;
  /* ../../rtl/core/neorv32_clint.vhd:185:33  */
  assign n2206 = mtime_en | n2204;
  /* ../../rtl/core/neorv32_package.vhd:1220:14  */
  assign n2214 = 1'b0 | mswi_en;
  /* ../../rtl/core/neorv32_clint.vhd:185:61  */
  assign n2216 = n2206 | n2214;
  /* ../../rtl/core/neorv32_clint.vhd:186:21  */
  assign n2219 = n2091[73]; // extract
  assign n2220 = n2217[33:2]; // extract
  /* ../../rtl/core/neorv32_clint.vhd:186:7  */
  assign n2221 = n2219 ? rdata : n2220;
  assign n2222 = n2217[1]; // extract
  assign n2223 = {n2221, n2222, n2216};
  assign n2228 = {n2119, n2115};
  assign n2229 = {n2150, n2146};
  assign n2230 = {n2161, n2156};
  /* ../../rtl/core/neorv32_clint.vhd:145:7  */
  assign n2231 = n2166 ? n2167 : mswi;
  /* ../../rtl/core/neorv32_clint.vhd:145:7  */
  always @(posedge clk_i or posedge n2163)
    if (n2163)
      n2232 <= 1'b0;
    else
      n2232 <= n2231;
  /* ../../rtl/core/neorv32_clint.vhd:183:5  */
  always @(posedge clk_i or posedge n2195)
    if (n2195)
      n2233 <= 34'b0000000000000000000000000000000000;
    else
      n2233 <= n2223;
endmodule

module neorv32_bus_io_switch_65536_2c202e980184e67cda5d0f34d3be5ef651a6fcca
  (input  clk_i,
   input  rstn_i,
   input  [4:0] \main_req_i_main_req_i[meta] ,
   input  [31:0] \main_req_i_main_req_i[addr] ,
   input  [31:0] \main_req_i_main_req_i[data] ,
   input  [3:0] \main_req_i_main_req_i[ben] ,
   input  \main_req_i_main_req_i[stb] ,
   input  \main_req_i_main_req_i[rw] ,
   input  \main_req_i_main_req_i[amo] ,
   input  [3:0] \main_req_i_main_req_i[amoop] ,
   input  \main_req_i_main_req_i[burst] ,
   input  \main_req_i_main_req_i[lock] ,
   input  \dev_00_rsp_i_dev_00_rsp_i[ack] ,
   input  \dev_00_rsp_i_dev_00_rsp_i[err] ,
   input  [31:0] \dev_00_rsp_i_dev_00_rsp_i[data] ,
   input  \dev_01_rsp_i_dev_01_rsp_i[ack] ,
   input  \dev_01_rsp_i_dev_01_rsp_i[err] ,
   input  [31:0] \dev_01_rsp_i_dev_01_rsp_i[data] ,
   input  \dev_02_rsp_i_dev_02_rsp_i[ack] ,
   input  \dev_02_rsp_i_dev_02_rsp_i[err] ,
   input  [31:0] \dev_02_rsp_i_dev_02_rsp_i[data] ,
   input  \dev_03_rsp_i_dev_03_rsp_i[ack] ,
   input  \dev_03_rsp_i_dev_03_rsp_i[err] ,
   input  [31:0] \dev_03_rsp_i_dev_03_rsp_i[data] ,
   input  \dev_04_rsp_i_dev_04_rsp_i[ack] ,
   input  \dev_04_rsp_i_dev_04_rsp_i[err] ,
   input  [31:0] \dev_04_rsp_i_dev_04_rsp_i[data] ,
   input  \dev_05_rsp_i_dev_05_rsp_i[ack] ,
   input  \dev_05_rsp_i_dev_05_rsp_i[err] ,
   input  [31:0] \dev_05_rsp_i_dev_05_rsp_i[data] ,
   input  \dev_06_rsp_i_dev_06_rsp_i[ack] ,
   input  \dev_06_rsp_i_dev_06_rsp_i[err] ,
   input  [31:0] \dev_06_rsp_i_dev_06_rsp_i[data] ,
   input  \dev_07_rsp_i_dev_07_rsp_i[ack] ,
   input  \dev_07_rsp_i_dev_07_rsp_i[err] ,
   input  [31:0] \dev_07_rsp_i_dev_07_rsp_i[data] ,
   input  \dev_08_rsp_i_dev_08_rsp_i[ack] ,
   input  \dev_08_rsp_i_dev_08_rsp_i[err] ,
   input  [31:0] \dev_08_rsp_i_dev_08_rsp_i[data] ,
   input  \dev_09_rsp_i_dev_09_rsp_i[ack] ,
   input  \dev_09_rsp_i_dev_09_rsp_i[err] ,
   input  [31:0] \dev_09_rsp_i_dev_09_rsp_i[data] ,
   input  \dev_10_rsp_i_dev_10_rsp_i[ack] ,
   input  \dev_10_rsp_i_dev_10_rsp_i[err] ,
   input  [31:0] \dev_10_rsp_i_dev_10_rsp_i[data] ,
   input  \dev_11_rsp_i_dev_11_rsp_i[ack] ,
   input  \dev_11_rsp_i_dev_11_rsp_i[err] ,
   input  [31:0] \dev_11_rsp_i_dev_11_rsp_i[data] ,
   input  \dev_12_rsp_i_dev_12_rsp_i[ack] ,
   input  \dev_12_rsp_i_dev_12_rsp_i[err] ,
   input  [31:0] \dev_12_rsp_i_dev_12_rsp_i[data] ,
   input  \dev_13_rsp_i_dev_13_rsp_i[ack] ,
   input  \dev_13_rsp_i_dev_13_rsp_i[err] ,
   input  [31:0] \dev_13_rsp_i_dev_13_rsp_i[data] ,
   input  \dev_14_rsp_i_dev_14_rsp_i[ack] ,
   input  \dev_14_rsp_i_dev_14_rsp_i[err] ,
   input  [31:0] \dev_14_rsp_i_dev_14_rsp_i[data] ,
   input  \dev_15_rsp_i_dev_15_rsp_i[ack] ,
   input  \dev_15_rsp_i_dev_15_rsp_i[err] ,
   input  [31:0] \dev_15_rsp_i_dev_15_rsp_i[data] ,
   input  \dev_16_rsp_i_dev_16_rsp_i[ack] ,
   input  \dev_16_rsp_i_dev_16_rsp_i[err] ,
   input  [31:0] \dev_16_rsp_i_dev_16_rsp_i[data] ,
   input  \dev_17_rsp_i_dev_17_rsp_i[ack] ,
   input  \dev_17_rsp_i_dev_17_rsp_i[err] ,
   input  [31:0] \dev_17_rsp_i_dev_17_rsp_i[data] ,
   input  \dev_18_rsp_i_dev_18_rsp_i[ack] ,
   input  \dev_18_rsp_i_dev_18_rsp_i[err] ,
   input  [31:0] \dev_18_rsp_i_dev_18_rsp_i[data] ,
   input  \dev_19_rsp_i_dev_19_rsp_i[ack] ,
   input  \dev_19_rsp_i_dev_19_rsp_i[err] ,
   input  [31:0] \dev_19_rsp_i_dev_19_rsp_i[data] ,
   input  \dev_20_rsp_i_dev_20_rsp_i[ack] ,
   input  \dev_20_rsp_i_dev_20_rsp_i[err] ,
   input  [31:0] \dev_20_rsp_i_dev_20_rsp_i[data] ,
   input  \dev_21_rsp_i_dev_21_rsp_i[ack] ,
   input  \dev_21_rsp_i_dev_21_rsp_i[err] ,
   input  [31:0] \dev_21_rsp_i_dev_21_rsp_i[data] ,
   input  \dev_22_rsp_i_dev_22_rsp_i[ack] ,
   input  \dev_22_rsp_i_dev_22_rsp_i[err] ,
   input  [31:0] \dev_22_rsp_i_dev_22_rsp_i[data] ,
   input  \dev_23_rsp_i_dev_23_rsp_i[ack] ,
   input  \dev_23_rsp_i_dev_23_rsp_i[err] ,
   input  [31:0] \dev_23_rsp_i_dev_23_rsp_i[data] ,
   input  \dev_24_rsp_i_dev_24_rsp_i[ack] ,
   input  \dev_24_rsp_i_dev_24_rsp_i[err] ,
   input  [31:0] \dev_24_rsp_i_dev_24_rsp_i[data] ,
   input  \dev_25_rsp_i_dev_25_rsp_i[ack] ,
   input  \dev_25_rsp_i_dev_25_rsp_i[err] ,
   input  [31:0] \dev_25_rsp_i_dev_25_rsp_i[data] ,
   input  \dev_26_rsp_i_dev_26_rsp_i[ack] ,
   input  \dev_26_rsp_i_dev_26_rsp_i[err] ,
   input  [31:0] \dev_26_rsp_i_dev_26_rsp_i[data] ,
   input  \dev_27_rsp_i_dev_27_rsp_i[ack] ,
   input  \dev_27_rsp_i_dev_27_rsp_i[err] ,
   input  [31:0] \dev_27_rsp_i_dev_27_rsp_i[data] ,
   input  \dev_28_rsp_i_dev_28_rsp_i[ack] ,
   input  \dev_28_rsp_i_dev_28_rsp_i[err] ,
   input  [31:0] \dev_28_rsp_i_dev_28_rsp_i[data] ,
   input  \dev_29_rsp_i_dev_29_rsp_i[ack] ,
   input  \dev_29_rsp_i_dev_29_rsp_i[err] ,
   input  [31:0] \dev_29_rsp_i_dev_29_rsp_i[data] ,
   input  \dev_30_rsp_i_dev_30_rsp_i[ack] ,
   input  \dev_30_rsp_i_dev_30_rsp_i[err] ,
   input  [31:0] \dev_30_rsp_i_dev_30_rsp_i[data] ,
   input  \dev_31_rsp_i_dev_31_rsp_i[ack] ,
   input  \dev_31_rsp_i_dev_31_rsp_i[err] ,
   input  [31:0] \dev_31_rsp_i_dev_31_rsp_i[data] ,
   output \main_rsp_o_main_rsp_o[ack] ,
   output \main_rsp_o_main_rsp_o[err] ,
   output [31:0] \main_rsp_o_main_rsp_o[data] ,
   output [4:0] \dev_00_req_o_dev_00_req_o[meta] ,
   output [31:0] \dev_00_req_o_dev_00_req_o[addr] ,
   output [31:0] \dev_00_req_o_dev_00_req_o[data] ,
   output [3:0] \dev_00_req_o_dev_00_req_o[ben] ,
   output \dev_00_req_o_dev_00_req_o[stb] ,
   output \dev_00_req_o_dev_00_req_o[rw] ,
   output \dev_00_req_o_dev_00_req_o[amo] ,
   output [3:0] \dev_00_req_o_dev_00_req_o[amoop] ,
   output \dev_00_req_o_dev_00_req_o[burst] ,
   output \dev_00_req_o_dev_00_req_o[lock] ,
   output [4:0] \dev_01_req_o_dev_01_req_o[meta] ,
   output [31:0] \dev_01_req_o_dev_01_req_o[addr] ,
   output [31:0] \dev_01_req_o_dev_01_req_o[data] ,
   output [3:0] \dev_01_req_o_dev_01_req_o[ben] ,
   output \dev_01_req_o_dev_01_req_o[stb] ,
   output \dev_01_req_o_dev_01_req_o[rw] ,
   output \dev_01_req_o_dev_01_req_o[amo] ,
   output [3:0] \dev_01_req_o_dev_01_req_o[amoop] ,
   output \dev_01_req_o_dev_01_req_o[burst] ,
   output \dev_01_req_o_dev_01_req_o[lock] ,
   output [4:0] \dev_02_req_o_dev_02_req_o[meta] ,
   output [31:0] \dev_02_req_o_dev_02_req_o[addr] ,
   output [31:0] \dev_02_req_o_dev_02_req_o[data] ,
   output [3:0] \dev_02_req_o_dev_02_req_o[ben] ,
   output \dev_02_req_o_dev_02_req_o[stb] ,
   output \dev_02_req_o_dev_02_req_o[rw] ,
   output \dev_02_req_o_dev_02_req_o[amo] ,
   output [3:0] \dev_02_req_o_dev_02_req_o[amoop] ,
   output \dev_02_req_o_dev_02_req_o[burst] ,
   output \dev_02_req_o_dev_02_req_o[lock] ,
   output [4:0] \dev_03_req_o_dev_03_req_o[meta] ,
   output [31:0] \dev_03_req_o_dev_03_req_o[addr] ,
   output [31:0] \dev_03_req_o_dev_03_req_o[data] ,
   output [3:0] \dev_03_req_o_dev_03_req_o[ben] ,
   output \dev_03_req_o_dev_03_req_o[stb] ,
   output \dev_03_req_o_dev_03_req_o[rw] ,
   output \dev_03_req_o_dev_03_req_o[amo] ,
   output [3:0] \dev_03_req_o_dev_03_req_o[amoop] ,
   output \dev_03_req_o_dev_03_req_o[burst] ,
   output \dev_03_req_o_dev_03_req_o[lock] ,
   output [4:0] \dev_04_req_o_dev_04_req_o[meta] ,
   output [31:0] \dev_04_req_o_dev_04_req_o[addr] ,
   output [31:0] \dev_04_req_o_dev_04_req_o[data] ,
   output [3:0] \dev_04_req_o_dev_04_req_o[ben] ,
   output \dev_04_req_o_dev_04_req_o[stb] ,
   output \dev_04_req_o_dev_04_req_o[rw] ,
   output \dev_04_req_o_dev_04_req_o[amo] ,
   output [3:0] \dev_04_req_o_dev_04_req_o[amoop] ,
   output \dev_04_req_o_dev_04_req_o[burst] ,
   output \dev_04_req_o_dev_04_req_o[lock] ,
   output [4:0] \dev_05_req_o_dev_05_req_o[meta] ,
   output [31:0] \dev_05_req_o_dev_05_req_o[addr] ,
   output [31:0] \dev_05_req_o_dev_05_req_o[data] ,
   output [3:0] \dev_05_req_o_dev_05_req_o[ben] ,
   output \dev_05_req_o_dev_05_req_o[stb] ,
   output \dev_05_req_o_dev_05_req_o[rw] ,
   output \dev_05_req_o_dev_05_req_o[amo] ,
   output [3:0] \dev_05_req_o_dev_05_req_o[amoop] ,
   output \dev_05_req_o_dev_05_req_o[burst] ,
   output \dev_05_req_o_dev_05_req_o[lock] ,
   output [4:0] \dev_06_req_o_dev_06_req_o[meta] ,
   output [31:0] \dev_06_req_o_dev_06_req_o[addr] ,
   output [31:0] \dev_06_req_o_dev_06_req_o[data] ,
   output [3:0] \dev_06_req_o_dev_06_req_o[ben] ,
   output \dev_06_req_o_dev_06_req_o[stb] ,
   output \dev_06_req_o_dev_06_req_o[rw] ,
   output \dev_06_req_o_dev_06_req_o[amo] ,
   output [3:0] \dev_06_req_o_dev_06_req_o[amoop] ,
   output \dev_06_req_o_dev_06_req_o[burst] ,
   output \dev_06_req_o_dev_06_req_o[lock] ,
   output [4:0] \dev_07_req_o_dev_07_req_o[meta] ,
   output [31:0] \dev_07_req_o_dev_07_req_o[addr] ,
   output [31:0] \dev_07_req_o_dev_07_req_o[data] ,
   output [3:0] \dev_07_req_o_dev_07_req_o[ben] ,
   output \dev_07_req_o_dev_07_req_o[stb] ,
   output \dev_07_req_o_dev_07_req_o[rw] ,
   output \dev_07_req_o_dev_07_req_o[amo] ,
   output [3:0] \dev_07_req_o_dev_07_req_o[amoop] ,
   output \dev_07_req_o_dev_07_req_o[burst] ,
   output \dev_07_req_o_dev_07_req_o[lock] ,
   output [4:0] \dev_08_req_o_dev_08_req_o[meta] ,
   output [31:0] \dev_08_req_o_dev_08_req_o[addr] ,
   output [31:0] \dev_08_req_o_dev_08_req_o[data] ,
   output [3:0] \dev_08_req_o_dev_08_req_o[ben] ,
   output \dev_08_req_o_dev_08_req_o[stb] ,
   output \dev_08_req_o_dev_08_req_o[rw] ,
   output \dev_08_req_o_dev_08_req_o[amo] ,
   output [3:0] \dev_08_req_o_dev_08_req_o[amoop] ,
   output \dev_08_req_o_dev_08_req_o[burst] ,
   output \dev_08_req_o_dev_08_req_o[lock] ,
   output [4:0] \dev_09_req_o_dev_09_req_o[meta] ,
   output [31:0] \dev_09_req_o_dev_09_req_o[addr] ,
   output [31:0] \dev_09_req_o_dev_09_req_o[data] ,
   output [3:0] \dev_09_req_o_dev_09_req_o[ben] ,
   output \dev_09_req_o_dev_09_req_o[stb] ,
   output \dev_09_req_o_dev_09_req_o[rw] ,
   output \dev_09_req_o_dev_09_req_o[amo] ,
   output [3:0] \dev_09_req_o_dev_09_req_o[amoop] ,
   output \dev_09_req_o_dev_09_req_o[burst] ,
   output \dev_09_req_o_dev_09_req_o[lock] ,
   output [4:0] \dev_10_req_o_dev_10_req_o[meta] ,
   output [31:0] \dev_10_req_o_dev_10_req_o[addr] ,
   output [31:0] \dev_10_req_o_dev_10_req_o[data] ,
   output [3:0] \dev_10_req_o_dev_10_req_o[ben] ,
   output \dev_10_req_o_dev_10_req_o[stb] ,
   output \dev_10_req_o_dev_10_req_o[rw] ,
   output \dev_10_req_o_dev_10_req_o[amo] ,
   output [3:0] \dev_10_req_o_dev_10_req_o[amoop] ,
   output \dev_10_req_o_dev_10_req_o[burst] ,
   output \dev_10_req_o_dev_10_req_o[lock] ,
   output [4:0] \dev_11_req_o_dev_11_req_o[meta] ,
   output [31:0] \dev_11_req_o_dev_11_req_o[addr] ,
   output [31:0] \dev_11_req_o_dev_11_req_o[data] ,
   output [3:0] \dev_11_req_o_dev_11_req_o[ben] ,
   output \dev_11_req_o_dev_11_req_o[stb] ,
   output \dev_11_req_o_dev_11_req_o[rw] ,
   output \dev_11_req_o_dev_11_req_o[amo] ,
   output [3:0] \dev_11_req_o_dev_11_req_o[amoop] ,
   output \dev_11_req_o_dev_11_req_o[burst] ,
   output \dev_11_req_o_dev_11_req_o[lock] ,
   output [4:0] \dev_12_req_o_dev_12_req_o[meta] ,
   output [31:0] \dev_12_req_o_dev_12_req_o[addr] ,
   output [31:0] \dev_12_req_o_dev_12_req_o[data] ,
   output [3:0] \dev_12_req_o_dev_12_req_o[ben] ,
   output \dev_12_req_o_dev_12_req_o[stb] ,
   output \dev_12_req_o_dev_12_req_o[rw] ,
   output \dev_12_req_o_dev_12_req_o[amo] ,
   output [3:0] \dev_12_req_o_dev_12_req_o[amoop] ,
   output \dev_12_req_o_dev_12_req_o[burst] ,
   output \dev_12_req_o_dev_12_req_o[lock] ,
   output [4:0] \dev_13_req_o_dev_13_req_o[meta] ,
   output [31:0] \dev_13_req_o_dev_13_req_o[addr] ,
   output [31:0] \dev_13_req_o_dev_13_req_o[data] ,
   output [3:0] \dev_13_req_o_dev_13_req_o[ben] ,
   output \dev_13_req_o_dev_13_req_o[stb] ,
   output \dev_13_req_o_dev_13_req_o[rw] ,
   output \dev_13_req_o_dev_13_req_o[amo] ,
   output [3:0] \dev_13_req_o_dev_13_req_o[amoop] ,
   output \dev_13_req_o_dev_13_req_o[burst] ,
   output \dev_13_req_o_dev_13_req_o[lock] ,
   output [4:0] \dev_14_req_o_dev_14_req_o[meta] ,
   output [31:0] \dev_14_req_o_dev_14_req_o[addr] ,
   output [31:0] \dev_14_req_o_dev_14_req_o[data] ,
   output [3:0] \dev_14_req_o_dev_14_req_o[ben] ,
   output \dev_14_req_o_dev_14_req_o[stb] ,
   output \dev_14_req_o_dev_14_req_o[rw] ,
   output \dev_14_req_o_dev_14_req_o[amo] ,
   output [3:0] \dev_14_req_o_dev_14_req_o[amoop] ,
   output \dev_14_req_o_dev_14_req_o[burst] ,
   output \dev_14_req_o_dev_14_req_o[lock] ,
   output [4:0] \dev_15_req_o_dev_15_req_o[meta] ,
   output [31:0] \dev_15_req_o_dev_15_req_o[addr] ,
   output [31:0] \dev_15_req_o_dev_15_req_o[data] ,
   output [3:0] \dev_15_req_o_dev_15_req_o[ben] ,
   output \dev_15_req_o_dev_15_req_o[stb] ,
   output \dev_15_req_o_dev_15_req_o[rw] ,
   output \dev_15_req_o_dev_15_req_o[amo] ,
   output [3:0] \dev_15_req_o_dev_15_req_o[amoop] ,
   output \dev_15_req_o_dev_15_req_o[burst] ,
   output \dev_15_req_o_dev_15_req_o[lock] ,
   output [4:0] \dev_16_req_o_dev_16_req_o[meta] ,
   output [31:0] \dev_16_req_o_dev_16_req_o[addr] ,
   output [31:0] \dev_16_req_o_dev_16_req_o[data] ,
   output [3:0] \dev_16_req_o_dev_16_req_o[ben] ,
   output \dev_16_req_o_dev_16_req_o[stb] ,
   output \dev_16_req_o_dev_16_req_o[rw] ,
   output \dev_16_req_o_dev_16_req_o[amo] ,
   output [3:0] \dev_16_req_o_dev_16_req_o[amoop] ,
   output \dev_16_req_o_dev_16_req_o[burst] ,
   output \dev_16_req_o_dev_16_req_o[lock] ,
   output [4:0] \dev_17_req_o_dev_17_req_o[meta] ,
   output [31:0] \dev_17_req_o_dev_17_req_o[addr] ,
   output [31:0] \dev_17_req_o_dev_17_req_o[data] ,
   output [3:0] \dev_17_req_o_dev_17_req_o[ben] ,
   output \dev_17_req_o_dev_17_req_o[stb] ,
   output \dev_17_req_o_dev_17_req_o[rw] ,
   output \dev_17_req_o_dev_17_req_o[amo] ,
   output [3:0] \dev_17_req_o_dev_17_req_o[amoop] ,
   output \dev_17_req_o_dev_17_req_o[burst] ,
   output \dev_17_req_o_dev_17_req_o[lock] ,
   output [4:0] \dev_18_req_o_dev_18_req_o[meta] ,
   output [31:0] \dev_18_req_o_dev_18_req_o[addr] ,
   output [31:0] \dev_18_req_o_dev_18_req_o[data] ,
   output [3:0] \dev_18_req_o_dev_18_req_o[ben] ,
   output \dev_18_req_o_dev_18_req_o[stb] ,
   output \dev_18_req_o_dev_18_req_o[rw] ,
   output \dev_18_req_o_dev_18_req_o[amo] ,
   output [3:0] \dev_18_req_o_dev_18_req_o[amoop] ,
   output \dev_18_req_o_dev_18_req_o[burst] ,
   output \dev_18_req_o_dev_18_req_o[lock] ,
   output [4:0] \dev_19_req_o_dev_19_req_o[meta] ,
   output [31:0] \dev_19_req_o_dev_19_req_o[addr] ,
   output [31:0] \dev_19_req_o_dev_19_req_o[data] ,
   output [3:0] \dev_19_req_o_dev_19_req_o[ben] ,
   output \dev_19_req_o_dev_19_req_o[stb] ,
   output \dev_19_req_o_dev_19_req_o[rw] ,
   output \dev_19_req_o_dev_19_req_o[amo] ,
   output [3:0] \dev_19_req_o_dev_19_req_o[amoop] ,
   output \dev_19_req_o_dev_19_req_o[burst] ,
   output \dev_19_req_o_dev_19_req_o[lock] ,
   output [4:0] \dev_20_req_o_dev_20_req_o[meta] ,
   output [31:0] \dev_20_req_o_dev_20_req_o[addr] ,
   output [31:0] \dev_20_req_o_dev_20_req_o[data] ,
   output [3:0] \dev_20_req_o_dev_20_req_o[ben] ,
   output \dev_20_req_o_dev_20_req_o[stb] ,
   output \dev_20_req_o_dev_20_req_o[rw] ,
   output \dev_20_req_o_dev_20_req_o[amo] ,
   output [3:0] \dev_20_req_o_dev_20_req_o[amoop] ,
   output \dev_20_req_o_dev_20_req_o[burst] ,
   output \dev_20_req_o_dev_20_req_o[lock] ,
   output [4:0] \dev_21_req_o_dev_21_req_o[meta] ,
   output [31:0] \dev_21_req_o_dev_21_req_o[addr] ,
   output [31:0] \dev_21_req_o_dev_21_req_o[data] ,
   output [3:0] \dev_21_req_o_dev_21_req_o[ben] ,
   output \dev_21_req_o_dev_21_req_o[stb] ,
   output \dev_21_req_o_dev_21_req_o[rw] ,
   output \dev_21_req_o_dev_21_req_o[amo] ,
   output [3:0] \dev_21_req_o_dev_21_req_o[amoop] ,
   output \dev_21_req_o_dev_21_req_o[burst] ,
   output \dev_21_req_o_dev_21_req_o[lock] ,
   output [4:0] \dev_22_req_o_dev_22_req_o[meta] ,
   output [31:0] \dev_22_req_o_dev_22_req_o[addr] ,
   output [31:0] \dev_22_req_o_dev_22_req_o[data] ,
   output [3:0] \dev_22_req_o_dev_22_req_o[ben] ,
   output \dev_22_req_o_dev_22_req_o[stb] ,
   output \dev_22_req_o_dev_22_req_o[rw] ,
   output \dev_22_req_o_dev_22_req_o[amo] ,
   output [3:0] \dev_22_req_o_dev_22_req_o[amoop] ,
   output \dev_22_req_o_dev_22_req_o[burst] ,
   output \dev_22_req_o_dev_22_req_o[lock] ,
   output [4:0] \dev_23_req_o_dev_23_req_o[meta] ,
   output [31:0] \dev_23_req_o_dev_23_req_o[addr] ,
   output [31:0] \dev_23_req_o_dev_23_req_o[data] ,
   output [3:0] \dev_23_req_o_dev_23_req_o[ben] ,
   output \dev_23_req_o_dev_23_req_o[stb] ,
   output \dev_23_req_o_dev_23_req_o[rw] ,
   output \dev_23_req_o_dev_23_req_o[amo] ,
   output [3:0] \dev_23_req_o_dev_23_req_o[amoop] ,
   output \dev_23_req_o_dev_23_req_o[burst] ,
   output \dev_23_req_o_dev_23_req_o[lock] ,
   output [4:0] \dev_24_req_o_dev_24_req_o[meta] ,
   output [31:0] \dev_24_req_o_dev_24_req_o[addr] ,
   output [31:0] \dev_24_req_o_dev_24_req_o[data] ,
   output [3:0] \dev_24_req_o_dev_24_req_o[ben] ,
   output \dev_24_req_o_dev_24_req_o[stb] ,
   output \dev_24_req_o_dev_24_req_o[rw] ,
   output \dev_24_req_o_dev_24_req_o[amo] ,
   output [3:0] \dev_24_req_o_dev_24_req_o[amoop] ,
   output \dev_24_req_o_dev_24_req_o[burst] ,
   output \dev_24_req_o_dev_24_req_o[lock] ,
   output [4:0] \dev_25_req_o_dev_25_req_o[meta] ,
   output [31:0] \dev_25_req_o_dev_25_req_o[addr] ,
   output [31:0] \dev_25_req_o_dev_25_req_o[data] ,
   output [3:0] \dev_25_req_o_dev_25_req_o[ben] ,
   output \dev_25_req_o_dev_25_req_o[stb] ,
   output \dev_25_req_o_dev_25_req_o[rw] ,
   output \dev_25_req_o_dev_25_req_o[amo] ,
   output [3:0] \dev_25_req_o_dev_25_req_o[amoop] ,
   output \dev_25_req_o_dev_25_req_o[burst] ,
   output \dev_25_req_o_dev_25_req_o[lock] ,
   output [4:0] \dev_26_req_o_dev_26_req_o[meta] ,
   output [31:0] \dev_26_req_o_dev_26_req_o[addr] ,
   output [31:0] \dev_26_req_o_dev_26_req_o[data] ,
   output [3:0] \dev_26_req_o_dev_26_req_o[ben] ,
   output \dev_26_req_o_dev_26_req_o[stb] ,
   output \dev_26_req_o_dev_26_req_o[rw] ,
   output \dev_26_req_o_dev_26_req_o[amo] ,
   output [3:0] \dev_26_req_o_dev_26_req_o[amoop] ,
   output \dev_26_req_o_dev_26_req_o[burst] ,
   output \dev_26_req_o_dev_26_req_o[lock] ,
   output [4:0] \dev_27_req_o_dev_27_req_o[meta] ,
   output [31:0] \dev_27_req_o_dev_27_req_o[addr] ,
   output [31:0] \dev_27_req_o_dev_27_req_o[data] ,
   output [3:0] \dev_27_req_o_dev_27_req_o[ben] ,
   output \dev_27_req_o_dev_27_req_o[stb] ,
   output \dev_27_req_o_dev_27_req_o[rw] ,
   output \dev_27_req_o_dev_27_req_o[amo] ,
   output [3:0] \dev_27_req_o_dev_27_req_o[amoop] ,
   output \dev_27_req_o_dev_27_req_o[burst] ,
   output \dev_27_req_o_dev_27_req_o[lock] ,
   output [4:0] \dev_28_req_o_dev_28_req_o[meta] ,
   output [31:0] \dev_28_req_o_dev_28_req_o[addr] ,
   output [31:0] \dev_28_req_o_dev_28_req_o[data] ,
   output [3:0] \dev_28_req_o_dev_28_req_o[ben] ,
   output \dev_28_req_o_dev_28_req_o[stb] ,
   output \dev_28_req_o_dev_28_req_o[rw] ,
   output \dev_28_req_o_dev_28_req_o[amo] ,
   output [3:0] \dev_28_req_o_dev_28_req_o[amoop] ,
   output \dev_28_req_o_dev_28_req_o[burst] ,
   output \dev_28_req_o_dev_28_req_o[lock] ,
   output [4:0] \dev_29_req_o_dev_29_req_o[meta] ,
   output [31:0] \dev_29_req_o_dev_29_req_o[addr] ,
   output [31:0] \dev_29_req_o_dev_29_req_o[data] ,
   output [3:0] \dev_29_req_o_dev_29_req_o[ben] ,
   output \dev_29_req_o_dev_29_req_o[stb] ,
   output \dev_29_req_o_dev_29_req_o[rw] ,
   output \dev_29_req_o_dev_29_req_o[amo] ,
   output [3:0] \dev_29_req_o_dev_29_req_o[amoop] ,
   output \dev_29_req_o_dev_29_req_o[burst] ,
   output \dev_29_req_o_dev_29_req_o[lock] ,
   output [4:0] \dev_30_req_o_dev_30_req_o[meta] ,
   output [31:0] \dev_30_req_o_dev_30_req_o[addr] ,
   output [31:0] \dev_30_req_o_dev_30_req_o[data] ,
   output [3:0] \dev_30_req_o_dev_30_req_o[ben] ,
   output \dev_30_req_o_dev_30_req_o[stb] ,
   output \dev_30_req_o_dev_30_req_o[rw] ,
   output \dev_30_req_o_dev_30_req_o[amo] ,
   output [3:0] \dev_30_req_o_dev_30_req_o[amoop] ,
   output \dev_30_req_o_dev_30_req_o[burst] ,
   output \dev_30_req_o_dev_30_req_o[lock] ,
   output [4:0] \dev_31_req_o_dev_31_req_o[meta] ,
   output [31:0] \dev_31_req_o_dev_31_req_o[addr] ,
   output [31:0] \dev_31_req_o_dev_31_req_o[data] ,
   output [3:0] \dev_31_req_o_dev_31_req_o[ben] ,
   output \dev_31_req_o_dev_31_req_o[stb] ,
   output \dev_31_req_o_dev_31_req_o[rw] ,
   output \dev_31_req_o_dev_31_req_o[amo] ,
   output [3:0] \dev_31_req_o_dev_31_req_o[amoop] ,
   output \dev_31_req_o_dev_31_req_o[burst] ,
   output \dev_31_req_o_dev_31_req_o[lock] );
  wire [81:0] n1592;
  wire n1594;
  wire n1595;
  wire [31:0] n1596;
  wire [4:0] n1598;
  wire [31:0] n1599;
  wire [31:0] n1600;
  wire [3:0] n1601;
  wire n1602;
  wire n1603;
  wire n1604;
  wire [3:0] n1605;
  wire n1606;
  wire n1607;
  wire [33:0] n1608;
  wire [4:0] n1610;
  wire [31:0] n1611;
  wire [31:0] n1612;
  wire [3:0] n1613;
  wire n1614;
  wire n1615;
  wire n1616;
  wire [3:0] n1617;
  wire n1618;
  wire n1619;
  wire [33:0] n1620;
  wire [4:0] n1622;
  wire [31:0] n1623;
  wire [31:0] n1624;
  wire [3:0] n1625;
  wire n1626;
  wire n1627;
  wire n1628;
  wire [3:0] n1629;
  wire n1630;
  wire n1631;
  wire [33:0] n1632;
  wire [4:0] n1634;
  wire [31:0] n1635;
  wire [31:0] n1636;
  wire [3:0] n1637;
  wire n1638;
  wire n1639;
  wire n1640;
  wire [3:0] n1641;
  wire n1642;
  wire n1643;
  wire [33:0] n1644;
  wire [4:0] n1646;
  wire [31:0] n1647;
  wire [31:0] n1648;
  wire [3:0] n1649;
  wire n1650;
  wire n1651;
  wire n1652;
  wire [3:0] n1653;
  wire n1654;
  wire n1655;
  wire [33:0] n1656;
  wire [4:0] n1658;
  wire [31:0] n1659;
  wire [31:0] n1660;
  wire [3:0] n1661;
  wire n1662;
  wire n1663;
  wire n1664;
  wire [3:0] n1665;
  wire n1666;
  wire n1667;
  wire [33:0] n1668;
  wire [4:0] n1670;
  wire [31:0] n1671;
  wire [31:0] n1672;
  wire [3:0] n1673;
  wire n1674;
  wire n1675;
  wire n1676;
  wire [3:0] n1677;
  wire n1678;
  wire n1679;
  wire [33:0] n1680;
  wire [4:0] n1682;
  wire [31:0] n1683;
  wire [31:0] n1684;
  wire [3:0] n1685;
  wire n1686;
  wire n1687;
  wire n1688;
  wire [3:0] n1689;
  wire n1690;
  wire n1691;
  wire [33:0] n1692;
  wire [4:0] n1694;
  wire [31:0] n1695;
  wire [31:0] n1696;
  wire [3:0] n1697;
  wire n1698;
  wire n1699;
  wire n1700;
  wire [3:0] n1701;
  wire n1702;
  wire n1703;
  wire [33:0] n1704;
  wire [4:0] n1706;
  wire [31:0] n1707;
  wire [31:0] n1708;
  wire [3:0] n1709;
  wire n1710;
  wire n1711;
  wire n1712;
  wire [3:0] n1713;
  wire n1714;
  wire n1715;
  wire [33:0] n1716;
  wire [4:0] n1718;
  wire [31:0] n1719;
  wire [31:0] n1720;
  wire [3:0] n1721;
  wire n1722;
  wire n1723;
  wire n1724;
  wire [3:0] n1725;
  wire n1726;
  wire n1727;
  wire [33:0] n1728;
  wire [4:0] n1730;
  wire [31:0] n1731;
  wire [31:0] n1732;
  wire [3:0] n1733;
  wire n1734;
  wire n1735;
  wire n1736;
  wire [3:0] n1737;
  wire n1738;
  wire n1739;
  wire [33:0] n1740;
  wire [4:0] n1742;
  wire [31:0] n1743;
  wire [31:0] n1744;
  wire [3:0] n1745;
  wire n1746;
  wire n1747;
  wire n1748;
  wire [3:0] n1749;
  wire n1750;
  wire n1751;
  wire [33:0] n1752;
  wire [4:0] n1754;
  wire [31:0] n1755;
  wire [31:0] n1756;
  wire [3:0] n1757;
  wire n1758;
  wire n1759;
  wire n1760;
  wire [3:0] n1761;
  wire n1762;
  wire n1763;
  wire [33:0] n1764;
  wire [4:0] n1766;
  wire [31:0] n1767;
  wire [31:0] n1768;
  wire [3:0] n1769;
  wire n1770;
  wire n1771;
  wire n1772;
  wire [3:0] n1773;
  wire n1774;
  wire n1775;
  wire [33:0] n1776;
  wire [4:0] n1778;
  wire [31:0] n1779;
  wire [31:0] n1780;
  wire [3:0] n1781;
  wire n1782;
  wire n1783;
  wire n1784;
  wire [3:0] n1785;
  wire n1786;
  wire n1787;
  wire [33:0] n1788;
  wire [4:0] n1790;
  wire [31:0] n1791;
  wire [31:0] n1792;
  wire [3:0] n1793;
  wire n1794;
  wire n1795;
  wire n1796;
  wire [3:0] n1797;
  wire n1798;
  wire n1799;
  wire [33:0] n1800;
  wire [4:0] n1802;
  wire [31:0] n1803;
  wire [31:0] n1804;
  wire [3:0] n1805;
  wire n1806;
  wire n1807;
  wire n1808;
  wire [3:0] n1809;
  wire n1810;
  wire n1811;
  wire [33:0] n1812;
  wire [4:0] n1814;
  wire [31:0] n1815;
  wire [31:0] n1816;
  wire [3:0] n1817;
  wire n1818;
  wire n1819;
  wire n1820;
  wire [3:0] n1821;
  wire n1822;
  wire n1823;
  wire [33:0] n1824;
  wire [4:0] n1826;
  wire [31:0] n1827;
  wire [31:0] n1828;
  wire [3:0] n1829;
  wire n1830;
  wire n1831;
  wire n1832;
  wire [3:0] n1833;
  wire n1834;
  wire n1835;
  wire [33:0] n1836;
  wire [4:0] n1838;
  wire [31:0] n1839;
  wire [31:0] n1840;
  wire [3:0] n1841;
  wire n1842;
  wire n1843;
  wire n1844;
  wire [3:0] n1845;
  wire n1846;
  wire n1847;
  wire [33:0] n1848;
  wire [4:0] n1850;
  wire [31:0] n1851;
  wire [31:0] n1852;
  wire [3:0] n1853;
  wire n1854;
  wire n1855;
  wire n1856;
  wire [3:0] n1857;
  wire n1858;
  wire n1859;
  wire [33:0] n1860;
  wire [4:0] n1862;
  wire [31:0] n1863;
  wire [31:0] n1864;
  wire [3:0] n1865;
  wire n1866;
  wire n1867;
  wire n1868;
  wire [3:0] n1869;
  wire n1870;
  wire n1871;
  wire [33:0] n1872;
  wire [4:0] n1874;
  wire [31:0] n1875;
  wire [31:0] n1876;
  wire [3:0] n1877;
  wire n1878;
  wire n1879;
  wire n1880;
  wire [3:0] n1881;
  wire n1882;
  wire n1883;
  wire [33:0] n1884;
  wire [4:0] n1886;
  wire [31:0] n1887;
  wire [31:0] n1888;
  wire [3:0] n1889;
  wire n1890;
  wire n1891;
  wire n1892;
  wire [3:0] n1893;
  wire n1894;
  wire n1895;
  wire [33:0] n1896;
  wire [4:0] n1898;
  wire [31:0] n1899;
  wire [31:0] n1900;
  wire [3:0] n1901;
  wire n1902;
  wire n1903;
  wire n1904;
  wire [3:0] n1905;
  wire n1906;
  wire n1907;
  wire [33:0] n1908;
  wire [4:0] n1910;
  wire [31:0] n1911;
  wire [31:0] n1912;
  wire [3:0] n1913;
  wire n1914;
  wire n1915;
  wire n1916;
  wire [3:0] n1917;
  wire n1918;
  wire n1919;
  wire [33:0] n1920;
  wire [4:0] n1922;
  wire [31:0] n1923;
  wire [31:0] n1924;
  wire [3:0] n1925;
  wire n1926;
  wire n1927;
  wire n1928;
  wire [3:0] n1929;
  wire n1930;
  wire n1931;
  wire [33:0] n1932;
  wire [4:0] n1934;
  wire [31:0] n1935;
  wire [31:0] n1936;
  wire [3:0] n1937;
  wire n1938;
  wire n1939;
  wire n1940;
  wire [3:0] n1941;
  wire n1942;
  wire n1943;
  wire [33:0] n1944;
  wire [4:0] n1946;
  wire [31:0] n1947;
  wire [31:0] n1948;
  wire [3:0] n1949;
  wire n1950;
  wire n1951;
  wire n1952;
  wire [3:0] n1953;
  wire n1954;
  wire n1955;
  wire [33:0] n1956;
  wire [4:0] n1958;
  wire [31:0] n1959;
  wire [31:0] n1960;
  wire [3:0] n1961;
  wire n1962;
  wire n1963;
  wire n1964;
  wire [3:0] n1965;
  wire n1966;
  wire n1967;
  wire [33:0] n1968;
  wire [4:0] n1970;
  wire [31:0] n1971;
  wire [31:0] n1972;
  wire [3:0] n1973;
  wire n1974;
  wire n1975;
  wire n1976;
  wire [3:0] n1977;
  wire n1978;
  wire n1979;
  wire [33:0] n1980;
  wire [2623:0] dev_req;
  wire [1087:0] dev_rsp;
  wire [81:0] main_req;
  wire [33:0] main_rsp;
  wire \neorv32_bus_reg_inst.host_rsp_o_host_rsp_o[ack] ;
  wire \neorv32_bus_reg_inst.host_rsp_o_host_rsp_o[err] ;
  wire [31:0] \neorv32_bus_reg_inst.host_rsp_o_host_rsp_o[data] ;
  wire [4:0] \neorv32_bus_reg_inst.device_req_o_device_req_o[meta] ;
  wire [31:0] \neorv32_bus_reg_inst.device_req_o_device_req_o[addr] ;
  wire [31:0] \neorv32_bus_reg_inst.device_req_o_device_req_o[data] ;
  wire [3:0] \neorv32_bus_reg_inst.device_req_o_device_req_o[ben] ;
  wire \neorv32_bus_reg_inst.device_req_o_device_req_o[stb] ;
  wire \neorv32_bus_reg_inst.device_req_o_device_req_o[rw] ;
  wire \neorv32_bus_reg_inst.device_req_o_device_req_o[amo] ;
  wire [3:0] \neorv32_bus_reg_inst.device_req_o_device_req_o[amoop] ;
  wire \neorv32_bus_reg_inst.device_req_o_device_req_o[burst] ;
  wire \neorv32_bus_reg_inst.device_req_o_device_req_o[lock] ;
  wire [4:0] n1981;
  wire [31:0] n1982;
  wire [31:0] n1983;
  wire [3:0] n1984;
  wire n1985;
  wire n1986;
  wire n1987;
  wire [3:0] n1988;
  wire n1989;
  wire n1990;
  wire [33:0] n1991;
  wire [81:0] n1993;
  wire n1995;
  wire n1996;
  wire [31:0] n1997;
  wire [81:0] n1998;
  wire [81:0] n1999;
  wire [81:0] n2000;
  wire [81:0] n2001;
  wire [81:0] n2002;
  wire [81:0] n2003;
  wire [81:0] n2004;
  wire [81:0] n2005;
  wire [81:0] n2006;
  wire [81:0] n2007;
  wire [81:0] n2008;
  wire [81:0] n2009;
  wire [81:0] n2010;
  wire [81:0] n2011;
  wire [81:0] n2012;
  wire [81:0] n2013;
  wire [81:0] n2014;
  wire [81:0] n2015;
  wire [81:0] n2016;
  wire [81:0] n2017;
  wire [81:0] n2018;
  wire [81:0] n2019;
  wire [81:0] n2020;
  wire [81:0] n2021;
  wire [81:0] n2022;
  wire [81:0] n2023;
  wire [81:0] n2024;
  wire [81:0] n2025;
  wire [81:0] n2026;
  wire [81:0] n2027;
  wire [81:0] n2028;
  wire [81:0] n2029;
  wire [4:0] n2032;
  wire n2034;
  wire n2035;
  wire n2037;
  wire [7:0] n2038;
  wire [72:0] n2039;
  wire [4:0] n2042;
  wire n2044;
  wire n2045;
  wire n2047;
  wire [7:0] n2048;
  wire [72:0] n2049;
  localparam [33:0] n2053 = 34'b0000000000000000000000000000000000;
  wire [31:0] n2054;
  wire [31:0] n2056;
  wire [31:0] n2057;
  localparam [33:0] n2058 = 34'b0000000000000000000000000000000000;
  wire [1:0] n2059;
  wire [33:0] n2060;
  wire n2061;
  wire n2063;
  wire n2064;
  wire n2065;
  wire [33:0] n2066;
  wire n2067;
  wire n2069;
  wire n2070;
  wire [33:0] n2071;
  wire [31:0] n2072;
  wire [31:0] n2074;
  wire [31:0] n2075;
  wire [33:0] n2076;
  wire n2077;
  wire n2079;
  wire n2080;
  wire [33:0] n2081;
  wire n2082;
  wire n2084;
  wire n2085;
  wire [33:0] n2086;
  wire [2623:0] n2089;
  wire [1087:0] n2090;
  assign \main_rsp_o_main_rsp_o[ack]  = n1594; //(module output)
  assign \main_rsp_o_main_rsp_o[err]  = n1595; //(module output)
  assign \main_rsp_o_main_rsp_o[data]  = n1596; //(module output)
  assign \dev_00_req_o_dev_00_req_o[meta]  = n1598; //(module output)
  assign \dev_00_req_o_dev_00_req_o[addr]  = n1599; //(module output)
  assign \dev_00_req_o_dev_00_req_o[data]  = n1600; //(module output)
  assign \dev_00_req_o_dev_00_req_o[ben]  = n1601; //(module output)
  assign \dev_00_req_o_dev_00_req_o[stb]  = n1602; //(module output)
  assign \dev_00_req_o_dev_00_req_o[rw]  = n1603; //(module output)
  assign \dev_00_req_o_dev_00_req_o[amo]  = n1604; //(module output)
  assign \dev_00_req_o_dev_00_req_o[amoop]  = n1605; //(module output)
  assign \dev_00_req_o_dev_00_req_o[burst]  = n1606; //(module output)
  assign \dev_00_req_o_dev_00_req_o[lock]  = n1607; //(module output)
  assign \dev_01_req_o_dev_01_req_o[meta]  = n1610; //(module output)
  assign \dev_01_req_o_dev_01_req_o[addr]  = n1611; //(module output)
  assign \dev_01_req_o_dev_01_req_o[data]  = n1612; //(module output)
  assign \dev_01_req_o_dev_01_req_o[ben]  = n1613; //(module output)
  assign \dev_01_req_o_dev_01_req_o[stb]  = n1614; //(module output)
  assign \dev_01_req_o_dev_01_req_o[rw]  = n1615; //(module output)
  assign \dev_01_req_o_dev_01_req_o[amo]  = n1616; //(module output)
  assign \dev_01_req_o_dev_01_req_o[amoop]  = n1617; //(module output)
  assign \dev_01_req_o_dev_01_req_o[burst]  = n1618; //(module output)
  assign \dev_01_req_o_dev_01_req_o[lock]  = n1619; //(module output)
  assign \dev_02_req_o_dev_02_req_o[meta]  = n1622; //(module output)
  assign \dev_02_req_o_dev_02_req_o[addr]  = n1623; //(module output)
  assign \dev_02_req_o_dev_02_req_o[data]  = n1624; //(module output)
  assign \dev_02_req_o_dev_02_req_o[ben]  = n1625; //(module output)
  assign \dev_02_req_o_dev_02_req_o[stb]  = n1626; //(module output)
  assign \dev_02_req_o_dev_02_req_o[rw]  = n1627; //(module output)
  assign \dev_02_req_o_dev_02_req_o[amo]  = n1628; //(module output)
  assign \dev_02_req_o_dev_02_req_o[amoop]  = n1629; //(module output)
  assign \dev_02_req_o_dev_02_req_o[burst]  = n1630; //(module output)
  assign \dev_02_req_o_dev_02_req_o[lock]  = n1631; //(module output)
  assign \dev_03_req_o_dev_03_req_o[meta]  = n1634; //(module output)
  assign \dev_03_req_o_dev_03_req_o[addr]  = n1635; //(module output)
  assign \dev_03_req_o_dev_03_req_o[data]  = n1636; //(module output)
  assign \dev_03_req_o_dev_03_req_o[ben]  = n1637; //(module output)
  assign \dev_03_req_o_dev_03_req_o[stb]  = n1638; //(module output)
  assign \dev_03_req_o_dev_03_req_o[rw]  = n1639; //(module output)
  assign \dev_03_req_o_dev_03_req_o[amo]  = n1640; //(module output)
  assign \dev_03_req_o_dev_03_req_o[amoop]  = n1641; //(module output)
  assign \dev_03_req_o_dev_03_req_o[burst]  = n1642; //(module output)
  assign \dev_03_req_o_dev_03_req_o[lock]  = n1643; //(module output)
  assign \dev_04_req_o_dev_04_req_o[meta]  = n1646; //(module output)
  assign \dev_04_req_o_dev_04_req_o[addr]  = n1647; //(module output)
  assign \dev_04_req_o_dev_04_req_o[data]  = n1648; //(module output)
  assign \dev_04_req_o_dev_04_req_o[ben]  = n1649; //(module output)
  assign \dev_04_req_o_dev_04_req_o[stb]  = n1650; //(module output)
  assign \dev_04_req_o_dev_04_req_o[rw]  = n1651; //(module output)
  assign \dev_04_req_o_dev_04_req_o[amo]  = n1652; //(module output)
  assign \dev_04_req_o_dev_04_req_o[amoop]  = n1653; //(module output)
  assign \dev_04_req_o_dev_04_req_o[burst]  = n1654; //(module output)
  assign \dev_04_req_o_dev_04_req_o[lock]  = n1655; //(module output)
  assign \dev_05_req_o_dev_05_req_o[meta]  = n1658; //(module output)
  assign \dev_05_req_o_dev_05_req_o[addr]  = n1659; //(module output)
  assign \dev_05_req_o_dev_05_req_o[data]  = n1660; //(module output)
  assign \dev_05_req_o_dev_05_req_o[ben]  = n1661; //(module output)
  assign \dev_05_req_o_dev_05_req_o[stb]  = n1662; //(module output)
  assign \dev_05_req_o_dev_05_req_o[rw]  = n1663; //(module output)
  assign \dev_05_req_o_dev_05_req_o[amo]  = n1664; //(module output)
  assign \dev_05_req_o_dev_05_req_o[amoop]  = n1665; //(module output)
  assign \dev_05_req_o_dev_05_req_o[burst]  = n1666; //(module output)
  assign \dev_05_req_o_dev_05_req_o[lock]  = n1667; //(module output)
  assign \dev_06_req_o_dev_06_req_o[meta]  = n1670; //(module output)
  assign \dev_06_req_o_dev_06_req_o[addr]  = n1671; //(module output)
  assign \dev_06_req_o_dev_06_req_o[data]  = n1672; //(module output)
  assign \dev_06_req_o_dev_06_req_o[ben]  = n1673; //(module output)
  assign \dev_06_req_o_dev_06_req_o[stb]  = n1674; //(module output)
  assign \dev_06_req_o_dev_06_req_o[rw]  = n1675; //(module output)
  assign \dev_06_req_o_dev_06_req_o[amo]  = n1676; //(module output)
  assign \dev_06_req_o_dev_06_req_o[amoop]  = n1677; //(module output)
  assign \dev_06_req_o_dev_06_req_o[burst]  = n1678; //(module output)
  assign \dev_06_req_o_dev_06_req_o[lock]  = n1679; //(module output)
  assign \dev_07_req_o_dev_07_req_o[meta]  = n1682; //(module output)
  assign \dev_07_req_o_dev_07_req_o[addr]  = n1683; //(module output)
  assign \dev_07_req_o_dev_07_req_o[data]  = n1684; //(module output)
  assign \dev_07_req_o_dev_07_req_o[ben]  = n1685; //(module output)
  assign \dev_07_req_o_dev_07_req_o[stb]  = n1686; //(module output)
  assign \dev_07_req_o_dev_07_req_o[rw]  = n1687; //(module output)
  assign \dev_07_req_o_dev_07_req_o[amo]  = n1688; //(module output)
  assign \dev_07_req_o_dev_07_req_o[amoop]  = n1689; //(module output)
  assign \dev_07_req_o_dev_07_req_o[burst]  = n1690; //(module output)
  assign \dev_07_req_o_dev_07_req_o[lock]  = n1691; //(module output)
  assign \dev_08_req_o_dev_08_req_o[meta]  = n1694; //(module output)
  assign \dev_08_req_o_dev_08_req_o[addr]  = n1695; //(module output)
  assign \dev_08_req_o_dev_08_req_o[data]  = n1696; //(module output)
  assign \dev_08_req_o_dev_08_req_o[ben]  = n1697; //(module output)
  assign \dev_08_req_o_dev_08_req_o[stb]  = n1698; //(module output)
  assign \dev_08_req_o_dev_08_req_o[rw]  = n1699; //(module output)
  assign \dev_08_req_o_dev_08_req_o[amo]  = n1700; //(module output)
  assign \dev_08_req_o_dev_08_req_o[amoop]  = n1701; //(module output)
  assign \dev_08_req_o_dev_08_req_o[burst]  = n1702; //(module output)
  assign \dev_08_req_o_dev_08_req_o[lock]  = n1703; //(module output)
  assign \dev_09_req_o_dev_09_req_o[meta]  = n1706; //(module output)
  assign \dev_09_req_o_dev_09_req_o[addr]  = n1707; //(module output)
  assign \dev_09_req_o_dev_09_req_o[data]  = n1708; //(module output)
  assign \dev_09_req_o_dev_09_req_o[ben]  = n1709; //(module output)
  assign \dev_09_req_o_dev_09_req_o[stb]  = n1710; //(module output)
  assign \dev_09_req_o_dev_09_req_o[rw]  = n1711; //(module output)
  assign \dev_09_req_o_dev_09_req_o[amo]  = n1712; //(module output)
  assign \dev_09_req_o_dev_09_req_o[amoop]  = n1713; //(module output)
  assign \dev_09_req_o_dev_09_req_o[burst]  = n1714; //(module output)
  assign \dev_09_req_o_dev_09_req_o[lock]  = n1715; //(module output)
  assign \dev_10_req_o_dev_10_req_o[meta]  = n1718; //(module output)
  assign \dev_10_req_o_dev_10_req_o[addr]  = n1719; //(module output)
  assign \dev_10_req_o_dev_10_req_o[data]  = n1720; //(module output)
  assign \dev_10_req_o_dev_10_req_o[ben]  = n1721; //(module output)
  assign \dev_10_req_o_dev_10_req_o[stb]  = n1722; //(module output)
  assign \dev_10_req_o_dev_10_req_o[rw]  = n1723; //(module output)
  assign \dev_10_req_o_dev_10_req_o[amo]  = n1724; //(module output)
  assign \dev_10_req_o_dev_10_req_o[amoop]  = n1725; //(module output)
  assign \dev_10_req_o_dev_10_req_o[burst]  = n1726; //(module output)
  assign \dev_10_req_o_dev_10_req_o[lock]  = n1727; //(module output)
  assign \dev_11_req_o_dev_11_req_o[meta]  = n1730; //(module output)
  assign \dev_11_req_o_dev_11_req_o[addr]  = n1731; //(module output)
  assign \dev_11_req_o_dev_11_req_o[data]  = n1732; //(module output)
  assign \dev_11_req_o_dev_11_req_o[ben]  = n1733; //(module output)
  assign \dev_11_req_o_dev_11_req_o[stb]  = n1734; //(module output)
  assign \dev_11_req_o_dev_11_req_o[rw]  = n1735; //(module output)
  assign \dev_11_req_o_dev_11_req_o[amo]  = n1736; //(module output)
  assign \dev_11_req_o_dev_11_req_o[amoop]  = n1737; //(module output)
  assign \dev_11_req_o_dev_11_req_o[burst]  = n1738; //(module output)
  assign \dev_11_req_o_dev_11_req_o[lock]  = n1739; //(module output)
  assign \dev_12_req_o_dev_12_req_o[meta]  = n1742; //(module output)
  assign \dev_12_req_o_dev_12_req_o[addr]  = n1743; //(module output)
  assign \dev_12_req_o_dev_12_req_o[data]  = n1744; //(module output)
  assign \dev_12_req_o_dev_12_req_o[ben]  = n1745; //(module output)
  assign \dev_12_req_o_dev_12_req_o[stb]  = n1746; //(module output)
  assign \dev_12_req_o_dev_12_req_o[rw]  = n1747; //(module output)
  assign \dev_12_req_o_dev_12_req_o[amo]  = n1748; //(module output)
  assign \dev_12_req_o_dev_12_req_o[amoop]  = n1749; //(module output)
  assign \dev_12_req_o_dev_12_req_o[burst]  = n1750; //(module output)
  assign \dev_12_req_o_dev_12_req_o[lock]  = n1751; //(module output)
  assign \dev_13_req_o_dev_13_req_o[meta]  = n1754; //(module output)
  assign \dev_13_req_o_dev_13_req_o[addr]  = n1755; //(module output)
  assign \dev_13_req_o_dev_13_req_o[data]  = n1756; //(module output)
  assign \dev_13_req_o_dev_13_req_o[ben]  = n1757; //(module output)
  assign \dev_13_req_o_dev_13_req_o[stb]  = n1758; //(module output)
  assign \dev_13_req_o_dev_13_req_o[rw]  = n1759; //(module output)
  assign \dev_13_req_o_dev_13_req_o[amo]  = n1760; //(module output)
  assign \dev_13_req_o_dev_13_req_o[amoop]  = n1761; //(module output)
  assign \dev_13_req_o_dev_13_req_o[burst]  = n1762; //(module output)
  assign \dev_13_req_o_dev_13_req_o[lock]  = n1763; //(module output)
  assign \dev_14_req_o_dev_14_req_o[meta]  = n1766; //(module output)
  assign \dev_14_req_o_dev_14_req_o[addr]  = n1767; //(module output)
  assign \dev_14_req_o_dev_14_req_o[data]  = n1768; //(module output)
  assign \dev_14_req_o_dev_14_req_o[ben]  = n1769; //(module output)
  assign \dev_14_req_o_dev_14_req_o[stb]  = n1770; //(module output)
  assign \dev_14_req_o_dev_14_req_o[rw]  = n1771; //(module output)
  assign \dev_14_req_o_dev_14_req_o[amo]  = n1772; //(module output)
  assign \dev_14_req_o_dev_14_req_o[amoop]  = n1773; //(module output)
  assign \dev_14_req_o_dev_14_req_o[burst]  = n1774; //(module output)
  assign \dev_14_req_o_dev_14_req_o[lock]  = n1775; //(module output)
  assign \dev_15_req_o_dev_15_req_o[meta]  = n1778; //(module output)
  assign \dev_15_req_o_dev_15_req_o[addr]  = n1779; //(module output)
  assign \dev_15_req_o_dev_15_req_o[data]  = n1780; //(module output)
  assign \dev_15_req_o_dev_15_req_o[ben]  = n1781; //(module output)
  assign \dev_15_req_o_dev_15_req_o[stb]  = n1782; //(module output)
  assign \dev_15_req_o_dev_15_req_o[rw]  = n1783; //(module output)
  assign \dev_15_req_o_dev_15_req_o[amo]  = n1784; //(module output)
  assign \dev_15_req_o_dev_15_req_o[amoop]  = n1785; //(module output)
  assign \dev_15_req_o_dev_15_req_o[burst]  = n1786; //(module output)
  assign \dev_15_req_o_dev_15_req_o[lock]  = n1787; //(module output)
  assign \dev_16_req_o_dev_16_req_o[meta]  = n1790; //(module output)
  assign \dev_16_req_o_dev_16_req_o[addr]  = n1791; //(module output)
  assign \dev_16_req_o_dev_16_req_o[data]  = n1792; //(module output)
  assign \dev_16_req_o_dev_16_req_o[ben]  = n1793; //(module output)
  assign \dev_16_req_o_dev_16_req_o[stb]  = n1794; //(module output)
  assign \dev_16_req_o_dev_16_req_o[rw]  = n1795; //(module output)
  assign \dev_16_req_o_dev_16_req_o[amo]  = n1796; //(module output)
  assign \dev_16_req_o_dev_16_req_o[amoop]  = n1797; //(module output)
  assign \dev_16_req_o_dev_16_req_o[burst]  = n1798; //(module output)
  assign \dev_16_req_o_dev_16_req_o[lock]  = n1799; //(module output)
  assign \dev_17_req_o_dev_17_req_o[meta]  = n1802; //(module output)
  assign \dev_17_req_o_dev_17_req_o[addr]  = n1803; //(module output)
  assign \dev_17_req_o_dev_17_req_o[data]  = n1804; //(module output)
  assign \dev_17_req_o_dev_17_req_o[ben]  = n1805; //(module output)
  assign \dev_17_req_o_dev_17_req_o[stb]  = n1806; //(module output)
  assign \dev_17_req_o_dev_17_req_o[rw]  = n1807; //(module output)
  assign \dev_17_req_o_dev_17_req_o[amo]  = n1808; //(module output)
  assign \dev_17_req_o_dev_17_req_o[amoop]  = n1809; //(module output)
  assign \dev_17_req_o_dev_17_req_o[burst]  = n1810; //(module output)
  assign \dev_17_req_o_dev_17_req_o[lock]  = n1811; //(module output)
  assign \dev_18_req_o_dev_18_req_o[meta]  = n1814; //(module output)
  assign \dev_18_req_o_dev_18_req_o[addr]  = n1815; //(module output)
  assign \dev_18_req_o_dev_18_req_o[data]  = n1816; //(module output)
  assign \dev_18_req_o_dev_18_req_o[ben]  = n1817; //(module output)
  assign \dev_18_req_o_dev_18_req_o[stb]  = n1818; //(module output)
  assign \dev_18_req_o_dev_18_req_o[rw]  = n1819; //(module output)
  assign \dev_18_req_o_dev_18_req_o[amo]  = n1820; //(module output)
  assign \dev_18_req_o_dev_18_req_o[amoop]  = n1821; //(module output)
  assign \dev_18_req_o_dev_18_req_o[burst]  = n1822; //(module output)
  assign \dev_18_req_o_dev_18_req_o[lock]  = n1823; //(module output)
  assign \dev_19_req_o_dev_19_req_o[meta]  = n1826; //(module output)
  assign \dev_19_req_o_dev_19_req_o[addr]  = n1827; //(module output)
  assign \dev_19_req_o_dev_19_req_o[data]  = n1828; //(module output)
  assign \dev_19_req_o_dev_19_req_o[ben]  = n1829; //(module output)
  assign \dev_19_req_o_dev_19_req_o[stb]  = n1830; //(module output)
  assign \dev_19_req_o_dev_19_req_o[rw]  = n1831; //(module output)
  assign \dev_19_req_o_dev_19_req_o[amo]  = n1832; //(module output)
  assign \dev_19_req_o_dev_19_req_o[amoop]  = n1833; //(module output)
  assign \dev_19_req_o_dev_19_req_o[burst]  = n1834; //(module output)
  assign \dev_19_req_o_dev_19_req_o[lock]  = n1835; //(module output)
  assign \dev_20_req_o_dev_20_req_o[meta]  = n1838; //(module output)
  assign \dev_20_req_o_dev_20_req_o[addr]  = n1839; //(module output)
  assign \dev_20_req_o_dev_20_req_o[data]  = n1840; //(module output)
  assign \dev_20_req_o_dev_20_req_o[ben]  = n1841; //(module output)
  assign \dev_20_req_o_dev_20_req_o[stb]  = n1842; //(module output)
  assign \dev_20_req_o_dev_20_req_o[rw]  = n1843; //(module output)
  assign \dev_20_req_o_dev_20_req_o[amo]  = n1844; //(module output)
  assign \dev_20_req_o_dev_20_req_o[amoop]  = n1845; //(module output)
  assign \dev_20_req_o_dev_20_req_o[burst]  = n1846; //(module output)
  assign \dev_20_req_o_dev_20_req_o[lock]  = n1847; //(module output)
  assign \dev_21_req_o_dev_21_req_o[meta]  = n1850; //(module output)
  assign \dev_21_req_o_dev_21_req_o[addr]  = n1851; //(module output)
  assign \dev_21_req_o_dev_21_req_o[data]  = n1852; //(module output)
  assign \dev_21_req_o_dev_21_req_o[ben]  = n1853; //(module output)
  assign \dev_21_req_o_dev_21_req_o[stb]  = n1854; //(module output)
  assign \dev_21_req_o_dev_21_req_o[rw]  = n1855; //(module output)
  assign \dev_21_req_o_dev_21_req_o[amo]  = n1856; //(module output)
  assign \dev_21_req_o_dev_21_req_o[amoop]  = n1857; //(module output)
  assign \dev_21_req_o_dev_21_req_o[burst]  = n1858; //(module output)
  assign \dev_21_req_o_dev_21_req_o[lock]  = n1859; //(module output)
  assign \dev_22_req_o_dev_22_req_o[meta]  = n1862; //(module output)
  assign \dev_22_req_o_dev_22_req_o[addr]  = n1863; //(module output)
  assign \dev_22_req_o_dev_22_req_o[data]  = n1864; //(module output)
  assign \dev_22_req_o_dev_22_req_o[ben]  = n1865; //(module output)
  assign \dev_22_req_o_dev_22_req_o[stb]  = n1866; //(module output)
  assign \dev_22_req_o_dev_22_req_o[rw]  = n1867; //(module output)
  assign \dev_22_req_o_dev_22_req_o[amo]  = n1868; //(module output)
  assign \dev_22_req_o_dev_22_req_o[amoop]  = n1869; //(module output)
  assign \dev_22_req_o_dev_22_req_o[burst]  = n1870; //(module output)
  assign \dev_22_req_o_dev_22_req_o[lock]  = n1871; //(module output)
  assign \dev_23_req_o_dev_23_req_o[meta]  = n1874; //(module output)
  assign \dev_23_req_o_dev_23_req_o[addr]  = n1875; //(module output)
  assign \dev_23_req_o_dev_23_req_o[data]  = n1876; //(module output)
  assign \dev_23_req_o_dev_23_req_o[ben]  = n1877; //(module output)
  assign \dev_23_req_o_dev_23_req_o[stb]  = n1878; //(module output)
  assign \dev_23_req_o_dev_23_req_o[rw]  = n1879; //(module output)
  assign \dev_23_req_o_dev_23_req_o[amo]  = n1880; //(module output)
  assign \dev_23_req_o_dev_23_req_o[amoop]  = n1881; //(module output)
  assign \dev_23_req_o_dev_23_req_o[burst]  = n1882; //(module output)
  assign \dev_23_req_o_dev_23_req_o[lock]  = n1883; //(module output)
  assign \dev_24_req_o_dev_24_req_o[meta]  = n1886; //(module output)
  assign \dev_24_req_o_dev_24_req_o[addr]  = n1887; //(module output)
  assign \dev_24_req_o_dev_24_req_o[data]  = n1888; //(module output)
  assign \dev_24_req_o_dev_24_req_o[ben]  = n1889; //(module output)
  assign \dev_24_req_o_dev_24_req_o[stb]  = n1890; //(module output)
  assign \dev_24_req_o_dev_24_req_o[rw]  = n1891; //(module output)
  assign \dev_24_req_o_dev_24_req_o[amo]  = n1892; //(module output)
  assign \dev_24_req_o_dev_24_req_o[amoop]  = n1893; //(module output)
  assign \dev_24_req_o_dev_24_req_o[burst]  = n1894; //(module output)
  assign \dev_24_req_o_dev_24_req_o[lock]  = n1895; //(module output)
  assign \dev_25_req_o_dev_25_req_o[meta]  = n1898; //(module output)
  assign \dev_25_req_o_dev_25_req_o[addr]  = n1899; //(module output)
  assign \dev_25_req_o_dev_25_req_o[data]  = n1900; //(module output)
  assign \dev_25_req_o_dev_25_req_o[ben]  = n1901; //(module output)
  assign \dev_25_req_o_dev_25_req_o[stb]  = n1902; //(module output)
  assign \dev_25_req_o_dev_25_req_o[rw]  = n1903; //(module output)
  assign \dev_25_req_o_dev_25_req_o[amo]  = n1904; //(module output)
  assign \dev_25_req_o_dev_25_req_o[amoop]  = n1905; //(module output)
  assign \dev_25_req_o_dev_25_req_o[burst]  = n1906; //(module output)
  assign \dev_25_req_o_dev_25_req_o[lock]  = n1907; //(module output)
  assign \dev_26_req_o_dev_26_req_o[meta]  = n1910; //(module output)
  assign \dev_26_req_o_dev_26_req_o[addr]  = n1911; //(module output)
  assign \dev_26_req_o_dev_26_req_o[data]  = n1912; //(module output)
  assign \dev_26_req_o_dev_26_req_o[ben]  = n1913; //(module output)
  assign \dev_26_req_o_dev_26_req_o[stb]  = n1914; //(module output)
  assign \dev_26_req_o_dev_26_req_o[rw]  = n1915; //(module output)
  assign \dev_26_req_o_dev_26_req_o[amo]  = n1916; //(module output)
  assign \dev_26_req_o_dev_26_req_o[amoop]  = n1917; //(module output)
  assign \dev_26_req_o_dev_26_req_o[burst]  = n1918; //(module output)
  assign \dev_26_req_o_dev_26_req_o[lock]  = n1919; //(module output)
  assign \dev_27_req_o_dev_27_req_o[meta]  = n1922; //(module output)
  assign \dev_27_req_o_dev_27_req_o[addr]  = n1923; //(module output)
  assign \dev_27_req_o_dev_27_req_o[data]  = n1924; //(module output)
  assign \dev_27_req_o_dev_27_req_o[ben]  = n1925; //(module output)
  assign \dev_27_req_o_dev_27_req_o[stb]  = n1926; //(module output)
  assign \dev_27_req_o_dev_27_req_o[rw]  = n1927; //(module output)
  assign \dev_27_req_o_dev_27_req_o[amo]  = n1928; //(module output)
  assign \dev_27_req_o_dev_27_req_o[amoop]  = n1929; //(module output)
  assign \dev_27_req_o_dev_27_req_o[burst]  = n1930; //(module output)
  assign \dev_27_req_o_dev_27_req_o[lock]  = n1931; //(module output)
  assign \dev_28_req_o_dev_28_req_o[meta]  = n1934; //(module output)
  assign \dev_28_req_o_dev_28_req_o[addr]  = n1935; //(module output)
  assign \dev_28_req_o_dev_28_req_o[data]  = n1936; //(module output)
  assign \dev_28_req_o_dev_28_req_o[ben]  = n1937; //(module output)
  assign \dev_28_req_o_dev_28_req_o[stb]  = n1938; //(module output)
  assign \dev_28_req_o_dev_28_req_o[rw]  = n1939; //(module output)
  assign \dev_28_req_o_dev_28_req_o[amo]  = n1940; //(module output)
  assign \dev_28_req_o_dev_28_req_o[amoop]  = n1941; //(module output)
  assign \dev_28_req_o_dev_28_req_o[burst]  = n1942; //(module output)
  assign \dev_28_req_o_dev_28_req_o[lock]  = n1943; //(module output)
  assign \dev_29_req_o_dev_29_req_o[meta]  = n1946; //(module output)
  assign \dev_29_req_o_dev_29_req_o[addr]  = n1947; //(module output)
  assign \dev_29_req_o_dev_29_req_o[data]  = n1948; //(module output)
  assign \dev_29_req_o_dev_29_req_o[ben]  = n1949; //(module output)
  assign \dev_29_req_o_dev_29_req_o[stb]  = n1950; //(module output)
  assign \dev_29_req_o_dev_29_req_o[rw]  = n1951; //(module output)
  assign \dev_29_req_o_dev_29_req_o[amo]  = n1952; //(module output)
  assign \dev_29_req_o_dev_29_req_o[amoop]  = n1953; //(module output)
  assign \dev_29_req_o_dev_29_req_o[burst]  = n1954; //(module output)
  assign \dev_29_req_o_dev_29_req_o[lock]  = n1955; //(module output)
  assign \dev_30_req_o_dev_30_req_o[meta]  = n1958; //(module output)
  assign \dev_30_req_o_dev_30_req_o[addr]  = n1959; //(module output)
  assign \dev_30_req_o_dev_30_req_o[data]  = n1960; //(module output)
  assign \dev_30_req_o_dev_30_req_o[ben]  = n1961; //(module output)
  assign \dev_30_req_o_dev_30_req_o[stb]  = n1962; //(module output)
  assign \dev_30_req_o_dev_30_req_o[rw]  = n1963; //(module output)
  assign \dev_30_req_o_dev_30_req_o[amo]  = n1964; //(module output)
  assign \dev_30_req_o_dev_30_req_o[amoop]  = n1965; //(module output)
  assign \dev_30_req_o_dev_30_req_o[burst]  = n1966; //(module output)
  assign \dev_30_req_o_dev_30_req_o[lock]  = n1967; //(module output)
  assign \dev_31_req_o_dev_31_req_o[meta]  = n1970; //(module output)
  assign \dev_31_req_o_dev_31_req_o[addr]  = n1971; //(module output)
  assign \dev_31_req_o_dev_31_req_o[data]  = n1972; //(module output)
  assign \dev_31_req_o_dev_31_req_o[ben]  = n1973; //(module output)
  assign \dev_31_req_o_dev_31_req_o[stb]  = n1974; //(module output)
  assign \dev_31_req_o_dev_31_req_o[rw]  = n1975; //(module output)
  assign \dev_31_req_o_dev_31_req_o[amo]  = n1976; //(module output)
  assign \dev_31_req_o_dev_31_req_o[amoop]  = n1977; //(module output)
  assign \dev_31_req_o_dev_31_req_o[burst]  = n1978; //(module output)
  assign \dev_31_req_o_dev_31_req_o[lock]  = n1979; //(module output)
  assign n1592 = {\main_req_i_main_req_i[lock] , \main_req_i_main_req_i[burst] , \main_req_i_main_req_i[amoop] , \main_req_i_main_req_i[amo] , \main_req_i_main_req_i[rw] , \main_req_i_main_req_i[stb] , \main_req_i_main_req_i[ben] , \main_req_i_main_req_i[data] , \main_req_i_main_req_i[addr] , \main_req_i_main_req_i[meta] };
  assign n1594 = n1991[0]; // extract
  assign n1595 = n1991[1]; // extract
  assign n1596 = n1991[33:2]; // extract
  assign n1598 = n1998[4:0]; // extract
  assign n1599 = n1998[36:5]; // extract
  assign n1600 = n1998[68:37]; // extract
  assign n1601 = n1998[72:69]; // extract
  assign n1602 = n1998[73]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:75:5  */
  assign n1603 = n1998[74]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:70:3  */
  assign n1604 = n1998[75]; // extract
  assign n1605 = n1998[79:76]; // extract
  assign n1606 = n1998[80]; // extract
  assign n1607 = n1998[81]; // extract
  assign n1608 = {\dev_00_rsp_i_dev_00_rsp_i[data] , \dev_00_rsp_i_dev_00_rsp_i[err] , \dev_00_rsp_i_dev_00_rsp_i[ack] };
  assign n1610 = n1999[4:0]; // extract
  assign n1611 = n1999[36:5]; // extract
  assign n1612 = n1999[68:37]; // extract
  assign n1613 = n1999[72:69]; // extract
  assign n1614 = n1999[73]; // extract
  assign n1615 = n1999[74]; // extract
  assign n1616 = n1999[75]; // extract
  assign n1617 = n1999[79:76]; // extract
  assign n1618 = n1999[80]; // extract
  assign n1619 = n1999[81]; // extract
  assign n1620 = {\dev_01_rsp_i_dev_01_rsp_i[data] , \dev_01_rsp_i_dev_01_rsp_i[err] , \dev_01_rsp_i_dev_01_rsp_i[ack] };
  assign n1622 = n2000[4:0]; // extract
  assign n1623 = n2000[36:5]; // extract
  assign n1624 = n2000[68:37]; // extract
  assign n1625 = n2000[72:69]; // extract
  assign n1626 = n2000[73]; // extract
  assign n1627 = n2000[74]; // extract
  assign n1628 = n2000[75]; // extract
  assign n1629 = n2000[79:76]; // extract
  assign n1630 = n2000[80]; // extract
  assign n1631 = n2000[81]; // extract
  assign n1632 = {\dev_02_rsp_i_dev_02_rsp_i[data] , \dev_02_rsp_i_dev_02_rsp_i[err] , \dev_02_rsp_i_dev_02_rsp_i[ack] };
  assign n1634 = n2001[4:0]; // extract
  assign n1635 = n2001[36:5]; // extract
  assign n1636 = n2001[68:37]; // extract
  assign n1637 = n2001[72:69]; // extract
  assign n1638 = n2001[73]; // extract
  assign n1639 = n2001[74]; // extract
  assign n1640 = n2001[75]; // extract
  assign n1641 = n2001[79:76]; // extract
  assign n1642 = n2001[80]; // extract
  assign n1643 = n2001[81]; // extract
  assign n1644 = {\dev_03_rsp_i_dev_03_rsp_i[data] , \dev_03_rsp_i_dev_03_rsp_i[err] , \dev_03_rsp_i_dev_03_rsp_i[ack] };
  assign n1646 = n2002[4:0]; // extract
  assign n1647 = n2002[36:5]; // extract
  assign n1648 = n2002[68:37]; // extract
  assign n1649 = n2002[72:69]; // extract
  assign n1650 = n2002[73]; // extract
  assign n1651 = n2002[74]; // extract
  assign n1652 = n2002[75]; // extract
  assign n1653 = n2002[79:76]; // extract
  assign n1654 = n2002[80]; // extract
  assign n1655 = n2002[81]; // extract
  assign n1656 = {\dev_04_rsp_i_dev_04_rsp_i[data] , \dev_04_rsp_i_dev_04_rsp_i[err] , \dev_04_rsp_i_dev_04_rsp_i[ack] };
  assign n1658 = n2003[4:0]; // extract
  assign n1659 = n2003[36:5]; // extract
  assign n1660 = n2003[68:37]; // extract
  assign n1661 = n2003[72:69]; // extract
  assign n1662 = n2003[73]; // extract
  assign n1663 = n2003[74]; // extract
  assign n1664 = n2003[75]; // extract
  assign n1665 = n2003[79:76]; // extract
  assign n1666 = n2003[80]; // extract
  assign n1667 = n2003[81]; // extract
  assign n1668 = {\dev_05_rsp_i_dev_05_rsp_i[data] , \dev_05_rsp_i_dev_05_rsp_i[err] , \dev_05_rsp_i_dev_05_rsp_i[ack] };
  assign n1670 = n2004[4:0]; // extract
  assign n1671 = n2004[36:5]; // extract
  assign n1672 = n2004[68:37]; // extract
  assign n1673 = n2004[72:69]; // extract
  assign n1674 = n2004[73]; // extract
  assign n1675 = n2004[74]; // extract
  assign n1676 = n2004[75]; // extract
  assign n1677 = n2004[79:76]; // extract
  assign n1678 = n2004[80]; // extract
  assign n1679 = n2004[81]; // extract
  assign n1680 = {\dev_06_rsp_i_dev_06_rsp_i[data] , \dev_06_rsp_i_dev_06_rsp_i[err] , \dev_06_rsp_i_dev_06_rsp_i[ack] };
  assign n1682 = n2005[4:0]; // extract
  assign n1683 = n2005[36:5]; // extract
  assign n1684 = n2005[68:37]; // extract
  assign n1685 = n2005[72:69]; // extract
  assign n1686 = n2005[73]; // extract
  assign n1687 = n2005[74]; // extract
  assign n1688 = n2005[75]; // extract
  assign n1689 = n2005[79:76]; // extract
  assign n1690 = n2005[80]; // extract
  assign n1691 = n2005[81]; // extract
  assign n1692 = {\dev_07_rsp_i_dev_07_rsp_i[data] , \dev_07_rsp_i_dev_07_rsp_i[err] , \dev_07_rsp_i_dev_07_rsp_i[ack] };
  assign n1694 = n2006[4:0]; // extract
  assign n1695 = n2006[36:5]; // extract
  assign n1696 = n2006[68:37]; // extract
  assign n1697 = n2006[72:69]; // extract
  assign n1698 = n2006[73]; // extract
  assign n1699 = n2006[74]; // extract
  assign n1700 = n2006[75]; // extract
  assign n1701 = n2006[79:76]; // extract
  assign n1702 = n2006[80]; // extract
  assign n1703 = n2006[81]; // extract
  assign n1704 = {\dev_08_rsp_i_dev_08_rsp_i[data] , \dev_08_rsp_i_dev_08_rsp_i[err] , \dev_08_rsp_i_dev_08_rsp_i[ack] };
  assign n1706 = n2007[4:0]; // extract
  assign n1707 = n2007[36:5]; // extract
  assign n1708 = n2007[68:37]; // extract
  assign n1709 = n2007[72:69]; // extract
  assign n1710 = n2007[73]; // extract
  assign n1711 = n2007[74]; // extract
  assign n1712 = n2007[75]; // extract
  assign n1713 = n2007[79:76]; // extract
  assign n1714 = n2007[80]; // extract
  assign n1715 = n2007[81]; // extract
  assign n1716 = {\dev_09_rsp_i_dev_09_rsp_i[data] , \dev_09_rsp_i_dev_09_rsp_i[err] , \dev_09_rsp_i_dev_09_rsp_i[ack] };
  assign n1718 = n2008[4:0]; // extract
  assign n1719 = n2008[36:5]; // extract
  assign n1720 = n2008[68:37]; // extract
  assign n1721 = n2008[72:69]; // extract
  assign n1722 = n2008[73]; // extract
  assign n1723 = n2008[74]; // extract
  assign n1724 = n2008[75]; // extract
  assign n1725 = n2008[79:76]; // extract
  assign n1726 = n2008[80]; // extract
  assign n1727 = n2008[81]; // extract
  assign n1728 = {\dev_10_rsp_i_dev_10_rsp_i[data] , \dev_10_rsp_i_dev_10_rsp_i[err] , \dev_10_rsp_i_dev_10_rsp_i[ack] };
  assign n1730 = n2009[4:0]; // extract
  assign n1731 = n2009[36:5]; // extract
  assign n1732 = n2009[68:37]; // extract
  assign n1733 = n2009[72:69]; // extract
  assign n1734 = n2009[73]; // extract
  assign n1735 = n2009[74]; // extract
  assign n1736 = n2009[75]; // extract
  assign n1737 = n2009[79:76]; // extract
  assign n1738 = n2009[80]; // extract
  assign n1739 = n2009[81]; // extract
  assign n1740 = {\dev_11_rsp_i_dev_11_rsp_i[data] , \dev_11_rsp_i_dev_11_rsp_i[err] , \dev_11_rsp_i_dev_11_rsp_i[ack] };
  assign n1742 = n2010[4:0]; // extract
  assign n1743 = n2010[36:5]; // extract
  assign n1744 = n2010[68:37]; // extract
  assign n1745 = n2010[72:69]; // extract
  assign n1746 = n2010[73]; // extract
  assign n1747 = n2010[74]; // extract
  assign n1748 = n2010[75]; // extract
  assign n1749 = n2010[79:76]; // extract
  assign n1750 = n2010[80]; // extract
  assign n1751 = n2010[81]; // extract
  assign n1752 = {\dev_12_rsp_i_dev_12_rsp_i[data] , \dev_12_rsp_i_dev_12_rsp_i[err] , \dev_12_rsp_i_dev_12_rsp_i[ack] };
  assign n1754 = n2011[4:0]; // extract
  assign n1755 = n2011[36:5]; // extract
  assign n1756 = n2011[68:37]; // extract
  assign n1757 = n2011[72:69]; // extract
  assign n1758 = n2011[73]; // extract
  assign n1759 = n2011[74]; // extract
  assign n1760 = n2011[75]; // extract
  assign n1761 = n2011[79:76]; // extract
  assign n1762 = n2011[80]; // extract
  assign n1763 = n2011[81]; // extract
  assign n1764 = {\dev_13_rsp_i_dev_13_rsp_i[data] , \dev_13_rsp_i_dev_13_rsp_i[err] , \dev_13_rsp_i_dev_13_rsp_i[ack] };
  assign n1766 = n2012[4:0]; // extract
  assign n1767 = n2012[36:5]; // extract
  assign n1768 = n2012[68:37]; // extract
  assign n1769 = n2012[72:69]; // extract
  assign n1770 = n2012[73]; // extract
  assign n1771 = n2012[74]; // extract
  assign n1772 = n2012[75]; // extract
  assign n1773 = n2012[79:76]; // extract
  assign n1774 = n2012[80]; // extract
  assign n1775 = n2012[81]; // extract
  assign n1776 = {\dev_14_rsp_i_dev_14_rsp_i[data] , \dev_14_rsp_i_dev_14_rsp_i[err] , \dev_14_rsp_i_dev_14_rsp_i[ack] };
  assign n1778 = n2013[4:0]; // extract
  assign n1779 = n2013[36:5]; // extract
  assign n1780 = n2013[68:37]; // extract
  assign n1781 = n2013[72:69]; // extract
  assign n1782 = n2013[73]; // extract
  assign n1783 = n2013[74]; // extract
  assign n1784 = n2013[75]; // extract
  assign n1785 = n2013[79:76]; // extract
  assign n1786 = n2013[80]; // extract
  assign n1787 = n2013[81]; // extract
  assign n1788 = {\dev_15_rsp_i_dev_15_rsp_i[data] , \dev_15_rsp_i_dev_15_rsp_i[err] , \dev_15_rsp_i_dev_15_rsp_i[ack] };
  assign n1790 = n2014[4:0]; // extract
  assign n1791 = n2014[36:5]; // extract
  assign n1792 = n2014[68:37]; // extract
  assign n1793 = n2014[72:69]; // extract
  assign n1794 = n2014[73]; // extract
  assign n1795 = n2014[74]; // extract
  assign n1796 = n2014[75]; // extract
  assign n1797 = n2014[79:76]; // extract
  assign n1798 = n2014[80]; // extract
  assign n1799 = n2014[81]; // extract
  assign n1800 = {\dev_16_rsp_i_dev_16_rsp_i[data] , \dev_16_rsp_i_dev_16_rsp_i[err] , \dev_16_rsp_i_dev_16_rsp_i[ack] };
  assign n1802 = n2015[4:0]; // extract
  assign n1803 = n2015[36:5]; // extract
  assign n1804 = n2015[68:37]; // extract
  assign n1805 = n2015[72:69]; // extract
  assign n1806 = n2015[73]; // extract
  assign n1807 = n2015[74]; // extract
  assign n1808 = n2015[75]; // extract
  assign n1809 = n2015[79:76]; // extract
  assign n1810 = n2015[80]; // extract
  assign n1811 = n2015[81]; // extract
  assign n1812 = {\dev_17_rsp_i_dev_17_rsp_i[data] , \dev_17_rsp_i_dev_17_rsp_i[err] , \dev_17_rsp_i_dev_17_rsp_i[ack] };
  assign n1814 = n2016[4:0]; // extract
  assign n1815 = n2016[36:5]; // extract
  assign n1816 = n2016[68:37]; // extract
  assign n1817 = n2016[72:69]; // extract
  assign n1818 = n2016[73]; // extract
  assign n1819 = n2016[74]; // extract
  assign n1820 = n2016[75]; // extract
  assign n1821 = n2016[79:76]; // extract
  assign n1822 = n2016[80]; // extract
  assign n1823 = n2016[81]; // extract
  assign n1824 = {\dev_18_rsp_i_dev_18_rsp_i[data] , \dev_18_rsp_i_dev_18_rsp_i[err] , \dev_18_rsp_i_dev_18_rsp_i[ack] };
  assign n1826 = n2017[4:0]; // extract
  assign n1827 = n2017[36:5]; // extract
  assign n1828 = n2017[68:37]; // extract
  assign n1829 = n2017[72:69]; // extract
  assign n1830 = n2017[73]; // extract
  assign n1831 = n2017[74]; // extract
  assign n1832 = n2017[75]; // extract
  assign n1833 = n2017[79:76]; // extract
  assign n1834 = n2017[80]; // extract
  assign n1835 = n2017[81]; // extract
  assign n1836 = {\dev_19_rsp_i_dev_19_rsp_i[data] , \dev_19_rsp_i_dev_19_rsp_i[err] , \dev_19_rsp_i_dev_19_rsp_i[ack] };
  assign n1838 = n2018[4:0]; // extract
  assign n1839 = n2018[36:5]; // extract
  assign n1840 = n2018[68:37]; // extract
  assign n1841 = n2018[72:69]; // extract
  assign n1842 = n2018[73]; // extract
  assign n1843 = n2018[74]; // extract
  assign n1844 = n2018[75]; // extract
  assign n1845 = n2018[79:76]; // extract
  assign n1846 = n2018[80]; // extract
  assign n1847 = n2018[81]; // extract
  assign n1848 = {\dev_20_rsp_i_dev_20_rsp_i[data] , \dev_20_rsp_i_dev_20_rsp_i[err] , \dev_20_rsp_i_dev_20_rsp_i[ack] };
  assign n1850 = n2019[4:0]; // extract
  assign n1851 = n2019[36:5]; // extract
  assign n1852 = n2019[68:37]; // extract
  assign n1853 = n2019[72:69]; // extract
  assign n1854 = n2019[73]; // extract
  assign n1855 = n2019[74]; // extract
  assign n1856 = n2019[75]; // extract
  assign n1857 = n2019[79:76]; // extract
  assign n1858 = n2019[80]; // extract
  assign n1859 = n2019[81]; // extract
  assign n1860 = {\dev_21_rsp_i_dev_21_rsp_i[data] , \dev_21_rsp_i_dev_21_rsp_i[err] , \dev_21_rsp_i_dev_21_rsp_i[ack] };
  assign n1862 = n2020[4:0]; // extract
  assign n1863 = n2020[36:5]; // extract
  assign n1864 = n2020[68:37]; // extract
  assign n1865 = n2020[72:69]; // extract
  assign n1866 = n2020[73]; // extract
  assign n1867 = n2020[74]; // extract
  assign n1868 = n2020[75]; // extract
  assign n1869 = n2020[79:76]; // extract
  assign n1870 = n2020[80]; // extract
  assign n1871 = n2020[81]; // extract
  assign n1872 = {\dev_22_rsp_i_dev_22_rsp_i[data] , \dev_22_rsp_i_dev_22_rsp_i[err] , \dev_22_rsp_i_dev_22_rsp_i[ack] };
  assign n1874 = n2021[4:0]; // extract
  assign n1875 = n2021[36:5]; // extract
  assign n1876 = n2021[68:37]; // extract
  assign n1877 = n2021[72:69]; // extract
  assign n1878 = n2021[73]; // extract
  assign n1879 = n2021[74]; // extract
  assign n1880 = n2021[75]; // extract
  assign n1881 = n2021[79:76]; // extract
  assign n1882 = n2021[80]; // extract
  assign n1883 = n2021[81]; // extract
  assign n1884 = {\dev_23_rsp_i_dev_23_rsp_i[data] , \dev_23_rsp_i_dev_23_rsp_i[err] , \dev_23_rsp_i_dev_23_rsp_i[ack] };
  assign n1886 = n2022[4:0]; // extract
  assign n1887 = n2022[36:5]; // extract
  assign n1888 = n2022[68:37]; // extract
  assign n1889 = n2022[72:69]; // extract
  assign n1890 = n2022[73]; // extract
  assign n1891 = n2022[74]; // extract
  assign n1892 = n2022[75]; // extract
  assign n1893 = n2022[79:76]; // extract
  assign n1894 = n2022[80]; // extract
  assign n1895 = n2022[81]; // extract
  assign n1896 = {\dev_24_rsp_i_dev_24_rsp_i[data] , \dev_24_rsp_i_dev_24_rsp_i[err] , \dev_24_rsp_i_dev_24_rsp_i[ack] };
  assign n1898 = n2023[4:0]; // extract
  assign n1899 = n2023[36:5]; // extract
  assign n1900 = n2023[68:37]; // extract
  assign n1901 = n2023[72:69]; // extract
  assign n1902 = n2023[73]; // extract
  assign n1903 = n2023[74]; // extract
  assign n1904 = n2023[75]; // extract
  assign n1905 = n2023[79:76]; // extract
  assign n1906 = n2023[80]; // extract
  assign n1907 = n2023[81]; // extract
  assign n1908 = {\dev_25_rsp_i_dev_25_rsp_i[data] , \dev_25_rsp_i_dev_25_rsp_i[err] , \dev_25_rsp_i_dev_25_rsp_i[ack] };
  assign n1910 = n2024[4:0]; // extract
  assign n1911 = n2024[36:5]; // extract
  assign n1912 = n2024[68:37]; // extract
  assign n1913 = n2024[72:69]; // extract
  assign n1914 = n2024[73]; // extract
  assign n1915 = n2024[74]; // extract
  assign n1916 = n2024[75]; // extract
  assign n1917 = n2024[79:76]; // extract
  assign n1918 = n2024[80]; // extract
  assign n1919 = n2024[81]; // extract
  assign n1920 = {\dev_26_rsp_i_dev_26_rsp_i[data] , \dev_26_rsp_i_dev_26_rsp_i[err] , \dev_26_rsp_i_dev_26_rsp_i[ack] };
  assign n1922 = n2025[4:0]; // extract
  assign n1923 = n2025[36:5]; // extract
  assign n1924 = n2025[68:37]; // extract
  assign n1925 = n2025[72:69]; // extract
  assign n1926 = n2025[73]; // extract
  assign n1927 = n2025[74]; // extract
  assign n1928 = n2025[75]; // extract
  assign n1929 = n2025[79:76]; // extract
  assign n1930 = n2025[80]; // extract
  assign n1931 = n2025[81]; // extract
  assign n1932 = {\dev_27_rsp_i_dev_27_rsp_i[data] , \dev_27_rsp_i_dev_27_rsp_i[err] , \dev_27_rsp_i_dev_27_rsp_i[ack] };
  assign n1934 = n2026[4:0]; // extract
  assign n1935 = n2026[36:5]; // extract
  assign n1936 = n2026[68:37]; // extract
  assign n1937 = n2026[72:69]; // extract
  assign n1938 = n2026[73]; // extract
  assign n1939 = n2026[74]; // extract
  assign n1940 = n2026[75]; // extract
  assign n1941 = n2026[79:76]; // extract
  assign n1942 = n2026[80]; // extract
  assign n1943 = n2026[81]; // extract
  assign n1944 = {\dev_28_rsp_i_dev_28_rsp_i[data] , \dev_28_rsp_i_dev_28_rsp_i[err] , \dev_28_rsp_i_dev_28_rsp_i[ack] };
  assign n1946 = n2027[4:0]; // extract
  assign n1947 = n2027[36:5]; // extract
  assign n1948 = n2027[68:37]; // extract
  assign n1949 = n2027[72:69]; // extract
  assign n1950 = n2027[73]; // extract
  assign n1951 = n2027[74]; // extract
  assign n1952 = n2027[75]; // extract
  assign n1953 = n2027[79:76]; // extract
  assign n1954 = n2027[80]; // extract
  assign n1955 = n2027[81]; // extract
  assign n1956 = {\dev_29_rsp_i_dev_29_rsp_i[data] , \dev_29_rsp_i_dev_29_rsp_i[err] , \dev_29_rsp_i_dev_29_rsp_i[ack] };
  assign n1958 = n2028[4:0]; // extract
  assign n1959 = n2028[36:5]; // extract
  assign n1960 = n2028[68:37]; // extract
  assign n1961 = n2028[72:69]; // extract
  assign n1962 = n2028[73]; // extract
  assign n1963 = n2028[74]; // extract
  assign n1964 = n2028[75]; // extract
  assign n1965 = n2028[79:76]; // extract
  assign n1966 = n2028[80]; // extract
  assign n1967 = n2028[81]; // extract
  assign n1968 = {\dev_30_rsp_i_dev_30_rsp_i[data] , \dev_30_rsp_i_dev_30_rsp_i[err] , \dev_30_rsp_i_dev_30_rsp_i[ack] };
  assign n1970 = n2029[4:0]; // extract
  assign n1971 = n2029[36:5]; // extract
  assign n1972 = n2029[68:37]; // extract
  assign n1973 = n2029[72:69]; // extract
  assign n1974 = n2029[73]; // extract
  assign n1975 = n2029[74]; // extract
  assign n1976 = n2029[75]; // extract
  assign n1977 = n2029[79:76]; // extract
  assign n1978 = n2029[80]; // extract
  assign n1979 = n2029[81]; // extract
  assign n1980 = {\dev_31_rsp_i_dev_31_rsp_i[data] , \dev_31_rsp_i_dev_31_rsp_i[err] , \dev_31_rsp_i_dev_31_rsp_i[ack] };
  /* ../../rtl/core/neorv32_bus.vhd:576:10  */
  assign dev_req = n2089; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:577:10  */
  assign dev_rsp = n2090; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:580:10  */
  assign main_req = n1993; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:581:10  */
  assign main_rsp = n2086; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:587:3  */
  neorv32_bus_reg_9159cb8bcee7fcb95582f140960cdae72788d326 neorv32_bus_reg_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\host_req_i_host_req_i[meta] (n1981),
    .\host_req_i_host_req_i[addr] (n1982),
    .\host_req_i_host_req_i[data] (n1983),
    .\host_req_i_host_req_i[ben] (n1984),
    .\host_req_i_host_req_i[stb] (n1985),
    .\host_req_i_host_req_i[rw] (n1986),
    .\host_req_i_host_req_i[amo] (n1987),
    .\host_req_i_host_req_i[amoop] (n1988),
    .\host_req_i_host_req_i[burst] (n1989),
    .\host_req_i_host_req_i[lock] (n1990),
    .\device_rsp_i_device_rsp_i[ack] (n1995),
    .\device_rsp_i_device_rsp_i[err] (n1996),
    .\device_rsp_i_device_rsp_i[data] (n1997),
    .\host_rsp_o_host_rsp_o[ack] (\neorv32_bus_reg_inst.host_rsp_o_host_rsp_o[ack] ),
    .\host_rsp_o_host_rsp_o[err] (\neorv32_bus_reg_inst.host_rsp_o_host_rsp_o[err] ),
    .\host_rsp_o_host_rsp_o[data] (\neorv32_bus_reg_inst.host_rsp_o_host_rsp_o[data] ),
    .\device_req_o_device_req_o[meta] (\neorv32_bus_reg_inst.device_req_o_device_req_o[meta] ),
    .\device_req_o_device_req_o[addr] (\neorv32_bus_reg_inst.device_req_o_device_req_o[addr] ),
    .\device_req_o_device_req_o[data] (\neorv32_bus_reg_inst.device_req_o_device_req_o[data] ),
    .\device_req_o_device_req_o[ben] (\neorv32_bus_reg_inst.device_req_o_device_req_o[ben] ),
    .\device_req_o_device_req_o[stb] (\neorv32_bus_reg_inst.device_req_o_device_req_o[stb] ),
    .\device_req_o_device_req_o[rw] (\neorv32_bus_reg_inst.device_req_o_device_req_o[rw] ),
    .\device_req_o_device_req_o[amo] (\neorv32_bus_reg_inst.device_req_o_device_req_o[amo] ),
    .\device_req_o_device_req_o[amoop] (\neorv32_bus_reg_inst.device_req_o_device_req_o[amoop] ),
    .\device_req_o_device_req_o[burst] (\neorv32_bus_reg_inst.device_req_o_device_req_o[burst] ),
    .\device_req_o_device_req_o[lock] (\neorv32_bus_reg_inst.device_req_o_device_req_o[lock] ));
  assign n1981 = n1592[4:0]; // extract
  assign n1982 = n1592[36:5]; // extract
  assign n1983 = n1592[68:37]; // extract
  assign n1984 = n1592[72:69]; // extract
  assign n1985 = n1592[73]; // extract
  assign n1986 = n1592[74]; // extract
  assign n1987 = n1592[75]; // extract
  assign n1988 = n1592[79:76]; // extract
  assign n1989 = n1592[80]; // extract
  assign n1990 = n1592[81]; // extract
  assign n1991 = {\neorv32_bus_reg_inst.host_rsp_o_host_rsp_o[data] , \neorv32_bus_reg_inst.host_rsp_o_host_rsp_o[err] , \neorv32_bus_reg_inst.host_rsp_o_host_rsp_o[ack] };
  assign n1993 = {\neorv32_bus_reg_inst.device_req_o_device_req_o[lock] , \neorv32_bus_reg_inst.device_req_o_device_req_o[burst] , \neorv32_bus_reg_inst.device_req_o_device_req_o[amoop] , \neorv32_bus_reg_inst.device_req_o_device_req_o[amo] , \neorv32_bus_reg_inst.device_req_o_device_req_o[rw] , \neorv32_bus_reg_inst.device_req_o_device_req_o[stb] , \neorv32_bus_reg_inst.device_req_o_device_req_o[ben] , \neorv32_bus_reg_inst.device_req_o_device_req_o[data] , \neorv32_bus_reg_inst.device_req_o_device_req_o[addr] , \neorv32_bus_reg_inst.device_req_o_device_req_o[meta] };
  assign n1995 = main_rsp[0]; // extract
  assign n1996 = main_rsp[1]; // extract
  assign n1997 = main_rsp[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:605:26  */
  assign n1998 = dev_req[2623:2542]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:606:26  */
  assign n1999 = dev_req[2541:2460]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:607:26  */
  assign n2000 = dev_req[2459:2378]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:608:26  */
  assign n2001 = dev_req[2377:2296]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:609:26  */
  assign n2002 = dev_req[2295:2214]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:610:26  */
  assign n2003 = dev_req[2213:2132]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:611:26  */
  assign n2004 = dev_req[2131:2050]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:612:26  */
  assign n2005 = dev_req[2049:1968]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:613:26  */
  assign n2006 = dev_req[1967:1886]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:614:26  */
  assign n2007 = dev_req[1885:1804]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:615:26  */
  assign n2008 = dev_req[1803:1722]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:616:26  */
  assign n2009 = dev_req[1721:1640]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:617:26  */
  assign n2010 = dev_req[1639:1558]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:618:26  */
  assign n2011 = dev_req[1557:1476]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:619:26  */
  assign n2012 = dev_req[1475:1394]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:620:26  */
  assign n2013 = dev_req[1393:1312]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:621:26  */
  assign n2014 = dev_req[1311:1230]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:622:26  */
  assign n2015 = dev_req[1229:1148]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:623:26  */
  assign n2016 = dev_req[1147:1066]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:624:26  */
  assign n2017 = dev_req[1065:984]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:625:26  */
  assign n2018 = dev_req[983:902]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:626:26  */
  assign n2019 = dev_req[901:820]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:627:26  */
  assign n2020 = dev_req[819:738]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:628:26  */
  assign n2021 = dev_req[737:656]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:629:26  */
  assign n2022 = dev_req[655:574]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:630:26  */
  assign n2023 = dev_req[573:492]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:631:26  */
  assign n2024 = dev_req[491:410]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:632:26  */
  assign n2025 = dev_req[409:328]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:633:26  */
  assign n2026 = dev_req[327:246]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:634:26  */
  assign n2027 = dev_req[245:164]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:635:26  */
  assign n2028 = dev_req[163:82]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:636:26  */
  assign n2029 = dev_req[81:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:648:26  */
  assign n2032 = main_req[25:21]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:648:55  */
  assign n2034 = n2032 == 5'b10100;
  /* ../../rtl/core/neorv32_bus.vhd:649:38  */
  assign n2035 = main_req[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:648:9  */
  assign n2037 = n2034 ? n2035 : 1'b0;
  assign n2038 = main_req[81:74]; // extract
  assign n2039 = main_req[72:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:648:26  */
  assign n2042 = main_req[25:21]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:648:55  */
  assign n2044 = n2042 == 5'b11110;
  /* ../../rtl/core/neorv32_bus.vhd:649:38  */
  assign n2045 = main_req[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:648:9  */
  assign n2047 = n2044 ? n2045 : 1'b0;
  assign n2048 = main_req[81:74]; // extract
  assign n2049 = main_req[72:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:671:29  */
  assign n2054 = n2053[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:671:48  */
  assign n2056 = dev_rsp[407:376]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:671:34  */
  assign n2057 = n2054 | n2056;
  assign n2059 = n2058[1:0]; // extract
  assign n2060 = {n2057, n2059};
  /* ../../rtl/core/neorv32_bus.vhd:672:29  */
  assign n2061 = n2060[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:672:48  */
  assign n2063 = dev_rsp[374]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:672:34  */
  assign n2064 = n2061 | n2063;
  assign n2065 = n2058[1]; // extract
  assign n2066 = {n2057, n2065, n2064};
  /* ../../rtl/core/neorv32_bus.vhd:673:29  */
  assign n2067 = n2066[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:673:48  */
  assign n2069 = dev_rsp[375]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:673:34  */
  assign n2070 = n2067 | n2069;
  assign n2071 = {n2057, n2070, n2064};
  /* ../../rtl/core/neorv32_bus.vhd:671:29  */
  assign n2072 = n2071[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:671:48  */
  assign n2074 = dev_rsp[67:36]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:671:34  */
  assign n2075 = n2072 | n2074;
  assign n2076 = {n2075, n2070, n2064};
  /* ../../rtl/core/neorv32_bus.vhd:672:29  */
  assign n2077 = n2076[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:672:48  */
  assign n2079 = dev_rsp[34]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:672:34  */
  assign n2080 = n2077 | n2079;
  assign n2081 = {n2075, n2070, n2080};
  /* ../../rtl/core/neorv32_bus.vhd:673:29  */
  assign n2082 = n2081[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:673:48  */
  assign n2084 = dev_rsp[35]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:673:34  */
  assign n2085 = n2082 | n2084;
  assign n2086 = {n2075, n2085, n2080};
  assign n2089 = {82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, n2038, n2037, n2039, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, n2048, n2047, n2049, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000};
  assign n2090 = {n1608, n1620, n1632, n1644, n1656, n1668, n1680, n1692, n1704, n1716, n1728, n1740, n1752, n1764, n1776, n1788, n1800, n1812, n1824, n1836, n1848, n1860, n1872, n1884, n1896, n1908, n1920, n1932, n1944, n1956, n1968, n1980};
endmodule

module neorv32_xbus_5ba93c9db0cff93f52b521d7420e43f6eda2784f
  (input  clk_i,
   input  rstn_i,
   input  bus_term_i,
   input  [4:0] \bus_req_i_bus_req_i[meta] ,
   input  [31:0] \bus_req_i_bus_req_i[addr] ,
   input  [31:0] \bus_req_i_bus_req_i[data] ,
   input  [3:0] \bus_req_i_bus_req_i[ben] ,
   input  \bus_req_i_bus_req_i[stb] ,
   input  \bus_req_i_bus_req_i[rw] ,
   input  \bus_req_i_bus_req_i[amo] ,
   input  [3:0] \bus_req_i_bus_req_i[amoop] ,
   input  \bus_req_i_bus_req_i[burst] ,
   input  \bus_req_i_bus_req_i[lock] ,
   input  [31:0] xbus_dat_i,
   input  xbus_ack_i,
   input  xbus_err_i,
   output \bus_rsp_o_bus_rsp_o[ack] ,
   output \bus_rsp_o_bus_rsp_o[err] ,
   output [31:0] \bus_rsp_o_bus_rsp_o[data] ,
   output [31:0] xbus_adr_o,
   output [31:0] xbus_dat_o,
   output [2:0] xbus_cti_o,
   output [2:0] xbus_tag_o,
   output xbus_we_o,
   output [3:0] xbus_sel_o,
   output xbus_stb_o,
   output xbus_cyc_o);
  wire [81:0] n1503;
  wire n1505;
  wire n1506;
  wire [31:0] n1507;
  wire [81:0] bus_req;
  wire [33:0] bus_rsp;
  wire pending;
  wire locked;
  wire \reg_stage_inst.host_rsp_o_host_rsp_o[ack] ;
  wire \reg_stage_inst.host_rsp_o_host_rsp_o[err] ;
  wire [31:0] \reg_stage_inst.host_rsp_o_host_rsp_o[data] ;
  wire [4:0] \reg_stage_inst.device_req_o_device_req_o[meta] ;
  wire [31:0] \reg_stage_inst.device_req_o_device_req_o[addr] ;
  wire [31:0] \reg_stage_inst.device_req_o_device_req_o[data] ;
  wire [3:0] \reg_stage_inst.device_req_o_device_req_o[ben] ;
  wire \reg_stage_inst.device_req_o_device_req_o[stb] ;
  wire \reg_stage_inst.device_req_o_device_req_o[rw] ;
  wire \reg_stage_inst.device_req_o_device_req_o[amo] ;
  wire [3:0] \reg_stage_inst.device_req_o_device_req_o[amoop] ;
  wire \reg_stage_inst.device_req_o_device_req_o[burst] ;
  wire \reg_stage_inst.device_req_o_device_req_o[lock] ;
  wire [4:0] n1516;
  wire [31:0] n1517;
  wire [31:0] n1518;
  wire [3:0] n1519;
  wire n1520;
  wire n1521;
  wire n1522;
  wire [3:0] n1523;
  wire n1524;
  wire n1525;
  wire [33:0] n1526;
  wire [81:0] n1528;
  wire n1530;
  wire n1531;
  wire [31:0] n1532;
  wire n1534;
  wire n1536;
  wire n1537;
  wire n1538;
  wire n1539;
  wire n1540;
  wire n1541;
  wire n1542;
  wire n1543;
  wire n1545;
  wire n1546;
  wire n1547;
  wire n1548;
  wire n1550;
  wire n1551;
  wire n1552;
  wire [31:0] n1561;
  wire [31:0] n1562;
  wire n1563;
  wire [3:0] n1564;
  wire n1565;
  wire n1566;
  wire n1567;
  wire n1569;
  wire n1570;
  wire n1571;
  wire [2:0] n1572;
  wire n1574;
  wire n1575;
  wire n1576;
  wire [2:0] n1577;
  wire n1579;
  wire n1581;
  wire [31:0] n1582;
  wire n1584;
  wire n1585;
  wire n1586;
  wire [33:0] n1587;
  reg n1588;
  wire n1589;
  reg n1590;
  wire [2:0] n1591;
  assign \bus_rsp_o_bus_rsp_o[ack]  = n1505; //(module output)
  assign \bus_rsp_o_bus_rsp_o[err]  = n1506; //(module output)
  assign \bus_rsp_o_bus_rsp_o[data]  = n1507; //(module output)
  assign xbus_adr_o = n1561; //(module output)
  assign xbus_dat_o = n1562; //(module output)
  assign xbus_cti_o = n1572; //(module output)
  assign xbus_tag_o = n1591; //(module output)
  assign xbus_we_o = n1563; //(module output)
  assign xbus_sel_o = n1564; //(module output)
  assign xbus_stb_o = n1565; //(module output)
  assign xbus_cyc_o = n1567; //(module output)
  assign n1503 = {\bus_req_i_bus_req_i[lock] , \bus_req_i_bus_req_i[burst] , \bus_req_i_bus_req_i[amoop] , \bus_req_i_bus_req_i[amo] , \bus_req_i_bus_req_i[rw] , \bus_req_i_bus_req_i[stb] , \bus_req_i_bus_req_i[ben] , \bus_req_i_bus_req_i[data] , \bus_req_i_bus_req_i[addr] , \bus_req_i_bus_req_i[meta] };
  assign n1505 = n1526[0]; // extract
  assign n1506 = n1526[1]; // extract
  assign n1507 = n1526[33:2]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:46:10  */
  assign bus_req = n1528; // (signal)
  /* ../../rtl/core/neorv32_xbus.vhd:47:10  */
  assign bus_rsp = n1587; // (signal)
  /* ../../rtl/core/neorv32_xbus.vhd:48:10  */
  assign pending = n1588; // (signal)
  /* ../../rtl/core/neorv32_xbus.vhd:48:19  */
  assign locked = n1590; // (signal)
  /* ../../rtl/core/neorv32_xbus.vhd:54:3  */
  neorv32_bus_reg_1489f923c4dca729178b3e3233458550d8dddf29 reg_stage_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\host_req_i_host_req_i[meta] (n1516),
    .\host_req_i_host_req_i[addr] (n1517),
    .\host_req_i_host_req_i[data] (n1518),
    .\host_req_i_host_req_i[ben] (n1519),
    .\host_req_i_host_req_i[stb] (n1520),
    .\host_req_i_host_req_i[rw] (n1521),
    .\host_req_i_host_req_i[amo] (n1522),
    .\host_req_i_host_req_i[amoop] (n1523),
    .\host_req_i_host_req_i[burst] (n1524),
    .\host_req_i_host_req_i[lock] (n1525),
    .\device_rsp_i_device_rsp_i[ack] (n1530),
    .\device_rsp_i_device_rsp_i[err] (n1531),
    .\device_rsp_i_device_rsp_i[data] (n1532),
    .\host_rsp_o_host_rsp_o[ack] (\reg_stage_inst.host_rsp_o_host_rsp_o[ack] ),
    .\host_rsp_o_host_rsp_o[err] (\reg_stage_inst.host_rsp_o_host_rsp_o[err] ),
    .\host_rsp_o_host_rsp_o[data] (\reg_stage_inst.host_rsp_o_host_rsp_o[data] ),
    .\device_req_o_device_req_o[meta] (\reg_stage_inst.device_req_o_device_req_o[meta] ),
    .\device_req_o_device_req_o[addr] (\reg_stage_inst.device_req_o_device_req_o[addr] ),
    .\device_req_o_device_req_o[data] (\reg_stage_inst.device_req_o_device_req_o[data] ),
    .\device_req_o_device_req_o[ben] (\reg_stage_inst.device_req_o_device_req_o[ben] ),
    .\device_req_o_device_req_o[stb] (\reg_stage_inst.device_req_o_device_req_o[stb] ),
    .\device_req_o_device_req_o[rw] (\reg_stage_inst.device_req_o_device_req_o[rw] ),
    .\device_req_o_device_req_o[amo] (\reg_stage_inst.device_req_o_device_req_o[amo] ),
    .\device_req_o_device_req_o[amoop] (\reg_stage_inst.device_req_o_device_req_o[amoop] ),
    .\device_req_o_device_req_o[burst] (\reg_stage_inst.device_req_o_device_req_o[burst] ),
    .\device_req_o_device_req_o[lock] (\reg_stage_inst.device_req_o_device_req_o[lock] ));
  /* ../../rtl/core/neorv32_bus.vhd:366:14  */
  assign n1516 = n1503[4:0]; // extract
  assign n1517 = n1503[36:5]; // extract
  assign n1518 = n1503[68:37]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:353:3  */
  assign n1519 = n1503[72:69]; // extract
  assign n1520 = n1503[73]; // extract
  assign n1521 = n1503[74]; // extract
  assign n1522 = n1503[75]; // extract
  assign n1523 = n1503[79:76]; // extract
  assign n1524 = n1503[80]; // extract
  assign n1525 = n1503[81]; // extract
  assign n1526 = {\reg_stage_inst.host_rsp_o_host_rsp_o[data] , \reg_stage_inst.host_rsp_o_host_rsp_o[err] , \reg_stage_inst.host_rsp_o_host_rsp_o[ack] };
  assign n1528 = {\reg_stage_inst.device_req_o_device_req_o[lock] , \reg_stage_inst.device_req_o_device_req_o[burst] , \reg_stage_inst.device_req_o_device_req_o[amoop] , \reg_stage_inst.device_req_o_device_req_o[amo] , \reg_stage_inst.device_req_o_device_req_o[rw] , \reg_stage_inst.device_req_o_device_req_o[stb] , \reg_stage_inst.device_req_o_device_req_o[ben] , \reg_stage_inst.device_req_o_device_req_o[data] , \reg_stage_inst.device_req_o_device_req_o[addr] , \reg_stage_inst.device_req_o_device_req_o[meta] };
  assign n1530 = bus_rsp[0]; // extract
  assign n1531 = bus_rsp[1]; // extract
  assign n1532 = bus_rsp[33:2]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:72:16  */
  assign n1534 = ~rstn_i;
  /* ../../rtl/core/neorv32_xbus.vhd:76:19  */
  assign n1536 = ~pending;
  /* ../../rtl/core/neorv32_xbus.vhd:77:28  */
  assign n1537 = bus_req[73]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:78:28  */
  assign n1538 = bus_req[73]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:78:44  */
  assign n1539 = bus_req[81]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:78:32  */
  assign n1540 = n1538 & n1539;
  /* ../../rtl/core/neorv32_xbus.vhd:80:20  */
  assign n1541 = ~locked;
  /* ../../rtl/core/neorv32_xbus.vhd:81:33  */
  assign n1542 = bus_term_i | xbus_err_i;
  /* ../../rtl/core/neorv32_xbus.vhd:81:55  */
  assign n1543 = n1542 | xbus_ack_i;
  /* ../../rtl/core/neorv32_xbus.vhd:81:11  */
  assign n1545 = n1543 ? 1'b0 : pending;
  /* ../../rtl/core/neorv32_xbus.vhd:85:45  */
  assign n1546 = bus_req[81]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:85:50  */
  assign n1547 = ~n1546;
  /* ../../rtl/core/neorv32_xbus.vhd:85:33  */
  assign n1548 = bus_term_i | n1547;
  /* ../../rtl/core/neorv32_xbus.vhd:85:11  */
  assign n1550 = n1548 ? 1'b0 : pending;
  /* ../../rtl/core/neorv32_xbus.vhd:80:9  */
  assign n1551 = n1541 ? n1545 : n1550;
  /* ../../rtl/core/neorv32_xbus.vhd:76:7  */
  assign n1552 = n1536 ? n1537 : n1551;
  /* ../../rtl/core/neorv32_xbus.vhd:95:25  */
  assign n1561 = bus_req[36:5]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:96:25  */
  assign n1562 = bus_req[68:37]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:97:25  */
  assign n1563 = bus_req[74]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:98:25  */
  assign n1564 = bus_req[72:69]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:99:25  */
  assign n1565 = bus_req[73]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:100:25  */
  assign n1566 = bus_req[73]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:100:29  */
  assign n1567 = n1566 | pending;
  /* ../../rtl/core/neorv32_xbus.vhd:103:37  */
  assign n1569 = bus_req[81]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:103:62  */
  assign n1570 = bus_req[75]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:103:49  */
  assign n1571 = n1570 & n1569;
  /* ../../rtl/core/neorv32_xbus.vhd:103:23  */
  assign n1572 = n1571 ? 3'b001 : n1577;
  /* ../../rtl/core/neorv32_xbus.vhd:104:37  */
  assign n1574 = bus_req[81]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:104:62  */
  assign n1575 = bus_req[80]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:104:49  */
  assign n1576 = n1575 & n1574;
  /* ../../rtl/core/neorv32_xbus.vhd:103:75  */
  assign n1577 = n1576 ? 3'b010 : 3'b000;
  /* ../../rtl/core/neorv32_xbus.vhd:108:32  */
  assign n1579 = bus_req[0]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:110:32  */
  assign n1581 = bus_req[1]; // extract
  /* ../../rtl/core/neorv32_xbus.vhd:113:30  */
  assign n1582 = pending ? xbus_dat_i : 32'b00000000000000000000000000000000;
  /* ../../rtl/core/neorv32_xbus.vhd:114:43  */
  assign n1584 = xbus_err_i | xbus_ack_i;
  /* ../../rtl/core/neorv32_xbus.vhd:114:27  */
  assign n1585 = pending & n1584;
  /* ../../rtl/core/neorv32_xbus.vhd:115:27  */
  assign n1586 = pending & xbus_err_i;
  assign n1587 = {n1582, n1586, n1585};
  /* ../../rtl/core/neorv32_xbus.vhd:75:5  */
  always @(posedge clk_i or posedge n1534)
    if (n1534)
      n1588 <= 1'b0;
    else
      n1588 <= n1552;
  /* ../../rtl/core/neorv32_xbus.vhd:75:5  */
  assign n1589 = n1536 ? n1540 : locked;
  /* ../../rtl/core/neorv32_xbus.vhd:75:5  */
  always @(posedge clk_i or posedge n1534)
    if (n1534)
      n1590 <= 1'b0;
    else
      n1590 <= n1589;
  /* ../../rtl/core/neorv32_xbus.vhd:72:5  */
  assign n1591 = {n1579, 1'b0, n1581};
endmodule

module neorv32_bus_gateway_16_0_16384_8192_2097152_31db79d4b6fe03934eb4ff891fa609cb3097642e
  (input  clk_i,
   input  rstn_i,
   input  [4:0] \req_i_req_i[meta] ,
   input  [31:0] \req_i_req_i[addr] ,
   input  [31:0] \req_i_req_i[data] ,
   input  [3:0] \req_i_req_i[ben] ,
   input  \req_i_req_i[stb] ,
   input  \req_i_req_i[rw] ,
   input  \req_i_req_i[amo] ,
   input  [3:0] \req_i_req_i[amoop] ,
   input  \req_i_req_i[burst] ,
   input  \req_i_req_i[lock] ,
   input  \a_rsp_i_a_rsp_i[ack] ,
   input  \a_rsp_i_a_rsp_i[err] ,
   input  [31:0] \a_rsp_i_a_rsp_i[data] ,
   input  \b_rsp_i_b_rsp_i[ack] ,
   input  \b_rsp_i_b_rsp_i[err] ,
   input  [31:0] \b_rsp_i_b_rsp_i[data] ,
   input  \c_rsp_i_c_rsp_i[ack] ,
   input  \c_rsp_i_c_rsp_i[err] ,
   input  [31:0] \c_rsp_i_c_rsp_i[data] ,
   input  \x_rsp_i_x_rsp_i[ack] ,
   input  \x_rsp_i_x_rsp_i[err] ,
   input  [31:0] \x_rsp_i_x_rsp_i[data] ,
   output term_o,
   output \rsp_o_rsp_o[ack] ,
   output \rsp_o_rsp_o[err] ,
   output [31:0] \rsp_o_rsp_o[data] ,
   output [4:0] \a_req_o_a_req_o[meta] ,
   output [31:0] \a_req_o_a_req_o[addr] ,
   output [31:0] \a_req_o_a_req_o[data] ,
   output [3:0] \a_req_o_a_req_o[ben] ,
   output \a_req_o_a_req_o[stb] ,
   output \a_req_o_a_req_o[rw] ,
   output \a_req_o_a_req_o[amo] ,
   output [3:0] \a_req_o_a_req_o[amoop] ,
   output \a_req_o_a_req_o[burst] ,
   output \a_req_o_a_req_o[lock] ,
   output [4:0] \b_req_o_b_req_o[meta] ,
   output [31:0] \b_req_o_b_req_o[addr] ,
   output [31:0] \b_req_o_b_req_o[data] ,
   output [3:0] \b_req_o_b_req_o[ben] ,
   output \b_req_o_b_req_o[stb] ,
   output \b_req_o_b_req_o[rw] ,
   output \b_req_o_b_req_o[amo] ,
   output [3:0] \b_req_o_b_req_o[amoop] ,
   output \b_req_o_b_req_o[burst] ,
   output \b_req_o_b_req_o[lock] ,
   output [4:0] \c_req_o_c_req_o[meta] ,
   output [31:0] \c_req_o_c_req_o[addr] ,
   output [31:0] \c_req_o_c_req_o[data] ,
   output [3:0] \c_req_o_c_req_o[ben] ,
   output \c_req_o_c_req_o[stb] ,
   output \c_req_o_c_req_o[rw] ,
   output \c_req_o_c_req_o[amo] ,
   output [3:0] \c_req_o_c_req_o[amoop] ,
   output \c_req_o_c_req_o[burst] ,
   output \c_req_o_c_req_o[lock] ,
   output [4:0] \x_req_o_x_req_o[meta] ,
   output [31:0] \x_req_o_x_req_o[addr] ,
   output [31:0] \x_req_o_x_req_o[data] ,
   output [3:0] \x_req_o_x_req_o[ben] ,
   output \x_req_o_x_req_o[stb] ,
   output \x_req_o_x_req_o[rw] ,
   output \x_req_o_x_req_o[amo] ,
   output [3:0] \x_req_o_x_req_o[amoop] ,
   output \x_req_o_x_req_o[burst] ,
   output \x_req_o_x_req_o[lock] );
  wire [81:0] n1287;
  wire n1289;
  wire n1290;
  wire [31:0] n1291;
  wire [4:0] n1293;
  wire [31:0] n1294;
  wire [31:0] n1295;
  wire [3:0] n1296;
  wire n1297;
  wire n1298;
  wire n1299;
  wire [3:0] n1300;
  wire n1301;
  wire n1302;
  wire [33:0] n1303;
  wire [4:0] n1305;
  wire [31:0] n1306;
  wire [31:0] n1307;
  wire [3:0] n1308;
  wire n1309;
  wire n1310;
  wire n1311;
  wire [3:0] n1312;
  wire n1313;
  wire n1314;
  wire [33:0] n1315;
  wire [4:0] n1317;
  wire [31:0] n1318;
  wire [31:0] n1319;
  wire [3:0] n1320;
  wire n1321;
  wire n1322;
  wire n1323;
  wire [3:0] n1324;
  wire n1325;
  wire n1326;
  wire [33:0] n1327;
  wire [4:0] n1329;
  wire [31:0] n1330;
  wire [31:0] n1331;
  wire [3:0] n1332;
  wire n1333;
  wire n1334;
  wire n1335;
  wire [3:0] n1336;
  wire n1337;
  wire n1338;
  wire [33:0] n1339;
  wire [3:0] port_sel;
  wire [327:0] port_req;
  wire [135:0] port_rsp;
  wire [33:0] int_rsp;
  wire [9:0] keeper;
  wire n1342;
  wire n1346;
  wire [10:0] n1349;
  wire n1351;
  wire n1353;
  wire n1354;
  wire [2:0] n1357;
  wire n1359;
  wire n1361;
  wire n1362;
  wire [81:0] n1364;
  wire [81:0] n1365;
  wire [81:0] n1366;
  wire [81:0] n1367;
  wire n1370;
  wire n1371;
  wire n1372;
  wire [7:0] n1373;
  wire [72:0] n1374;
  wire n1375;
  wire n1376;
  wire n1377;
  wire [7:0] n1378;
  wire [72:0] n1379;
  localparam [33:0] n1383 = 34'b0000000000000000000000000000000000;
  wire [31:0] n1384;
  wire [31:0] n1386;
  wire [31:0] n1387;
  localparam [33:0] n1388 = 34'b0000000000000000000000000000000000;
  wire [1:0] n1389;
  wire [33:0] n1390;
  wire n1391;
  wire n1393;
  wire n1394;
  wire n1395;
  wire [33:0] n1396;
  wire n1397;
  wire n1399;
  wire n1400;
  wire [33:0] n1401;
  wire [31:0] n1402;
  wire [31:0] n1404;
  wire [31:0] n1405;
  wire [33:0] n1406;
  wire n1407;
  wire n1409;
  wire n1410;
  wire [33:0] n1411;
  wire n1412;
  wire n1414;
  wire n1415;
  wire [33:0] n1416;
  wire [31:0] n1419;
  wire n1420;
  wire n1421;
  wire n1422;
  wire n1423;
  wire n1424;
  wire n1425;
  wire n1427;
  wire [1:0] n1433;
  wire n1434;
  wire n1435;
  wire n1437;
  wire [1:0] n1439;
  wire [1:0] n1440;
  wire n1442;
  wire n1443;
  wire [4:0] n1445;
  wire [4:0] n1447;
  wire [4:0] n1448;
  wire n1449;
  wire n1450;
  wire n1452;
  wire n1453;
  wire n1454;
  wire n1457;
  wire n1459;
  wire n1460;
  wire n1461;
  wire [1:0] n1463;
  wire [1:0] n1464;
  wire n1465;
  wire [1:0] n1467;
  wire [1:0] n1468;
  wire [1:0] n1469;
  wire [1:0] n1470;
  wire n1472;
  wire n1473;
  wire n1474;
  wire n1475;
  wire n1476;
  wire n1477;
  wire [1:0] n1479;
  wire [1:0] n1480;
  wire [1:0] n1481;
  reg [1:0] n1482;
  wire n1483;
  reg n1484;
  wire n1485;
  reg n1486;
  wire [4:0] n1487;
  reg [4:0] n1488;
  wire [8:0] n1489;
  wire [8:0] n1492;
  wire n1495;
  wire n1496;
  wire [3:0] n1497;
  wire [327:0] n1498;
  wire [135:0] n1499;
  reg [8:0] n1500;
  wire [9:0] n1501;
  wire [33:0] n1502;
  assign term_o = n1496; //(module output)
  assign \rsp_o_rsp_o[ack]  = n1289; //(module output)
  assign \rsp_o_rsp_o[err]  = n1290; //(module output)
  assign \rsp_o_rsp_o[data]  = n1291; //(module output)
  assign \a_req_o_a_req_o[meta]  = n1293; //(module output)
  assign \a_req_o_a_req_o[addr]  = n1294; //(module output)
  assign \a_req_o_a_req_o[data]  = n1295; //(module output)
  assign \a_req_o_a_req_o[ben]  = n1296; //(module output)
  assign \a_req_o_a_req_o[stb]  = n1297; //(module output)
  assign \a_req_o_a_req_o[rw]  = n1298; //(module output)
  assign \a_req_o_a_req_o[amo]  = n1299; //(module output)
  assign \a_req_o_a_req_o[amoop]  = n1300; //(module output)
  assign \a_req_o_a_req_o[burst]  = n1301; //(module output)
  assign \a_req_o_a_req_o[lock]  = n1302; //(module output)
  assign \b_req_o_b_req_o[meta]  = n1305; //(module output)
  assign \b_req_o_b_req_o[addr]  = n1306; //(module output)
  assign \b_req_o_b_req_o[data]  = n1307; //(module output)
  assign \b_req_o_b_req_o[ben]  = n1308; //(module output)
  assign \b_req_o_b_req_o[stb]  = n1309; //(module output)
  assign \b_req_o_b_req_o[rw]  = n1310; //(module output)
  assign \b_req_o_b_req_o[amo]  = n1311; //(module output)
  assign \b_req_o_b_req_o[amoop]  = n1312; //(module output)
  assign \b_req_o_b_req_o[burst]  = n1313; //(module output)
  assign \b_req_o_b_req_o[lock]  = n1314; //(module output)
  assign \c_req_o_c_req_o[meta]  = n1317; //(module output)
  assign \c_req_o_c_req_o[addr]  = n1318; //(module output)
  assign \c_req_o_c_req_o[data]  = n1319; //(module output)
  assign \c_req_o_c_req_o[ben]  = n1320; //(module output)
  assign \c_req_o_c_req_o[stb]  = n1321; //(module output)
  assign \c_req_o_c_req_o[rw]  = n1322; //(module output)
  assign \c_req_o_c_req_o[amo]  = n1323; //(module output)
  assign \c_req_o_c_req_o[amoop]  = n1324; //(module output)
  assign \c_req_o_c_req_o[burst]  = n1325; //(module output)
  assign \c_req_o_c_req_o[lock]  = n1326; //(module output)
  assign \x_req_o_x_req_o[meta]  = n1329; //(module output)
  assign \x_req_o_x_req_o[addr]  = n1330; //(module output)
  assign \x_req_o_x_req_o[data]  = n1331; //(module output)
  assign \x_req_o_x_req_o[ben]  = n1332; //(module output)
  assign \x_req_o_x_req_o[stb]  = n1333; //(module output)
  assign \x_req_o_x_req_o[rw]  = n1334; //(module output)
  assign \x_req_o_x_req_o[amo]  = n1335; //(module output)
  assign \x_req_o_x_req_o[amoop]  = n1336; //(module output)
  assign \x_req_o_x_req_o[burst]  = n1337; //(module output)
  assign \x_req_o_x_req_o[lock]  = n1338; //(module output)
  assign n1287 = {\req_i_req_i[lock] , \req_i_req_i[burst] , \req_i_req_i[amoop] , \req_i_req_i[amo] , \req_i_req_i[rw] , \req_i_req_i[stb] , \req_i_req_i[ben] , \req_i_req_i[data] , \req_i_req_i[addr] , \req_i_req_i[meta] };
  assign n1289 = n1502[0]; // extract
  assign n1290 = n1502[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:73:3  */
  assign n1291 = n1502[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:46:3  */
  assign n1293 = n1364[4:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:48:5  */
  assign n1294 = n1364[36:5]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:48:5  */
  assign n1295 = n1364[68:37]; // extract
  assign n1296 = n1364[72:69]; // extract
  assign n1297 = n1364[73]; // extract
  assign n1298 = n1364[74]; // extract
  assign n1299 = n1364[75]; // extract
  assign n1300 = n1364[79:76]; // extract
  assign n1301 = n1364[80]; // extract
  assign n1302 = n1364[81]; // extract
  assign n1303 = {\a_rsp_i_a_rsp_i[data] , \a_rsp_i_a_rsp_i[err] , \a_rsp_i_a_rsp_i[ack] };
  assign n1305 = n1365[4:0]; // extract
  assign n1306 = n1365[36:5]; // extract
  assign n1307 = n1365[68:37]; // extract
  assign n1308 = n1365[72:69]; // extract
  assign n1309 = n1365[73]; // extract
  assign n1310 = n1365[74]; // extract
  assign n1311 = n1365[75]; // extract
  assign n1312 = n1365[79:76]; // extract
  assign n1313 = n1365[80]; // extract
  assign n1314 = n1365[81]; // extract
  assign n1315 = {\b_rsp_i_b_rsp_i[data] , \b_rsp_i_b_rsp_i[err] , \b_rsp_i_b_rsp_i[ack] };
  assign n1317 = n1366[4:0]; // extract
  assign n1318 = n1366[36:5]; // extract
  assign n1319 = n1366[68:37]; // extract
  assign n1320 = n1366[72:69]; // extract
  assign n1321 = n1366[73]; // extract
  assign n1322 = n1366[74]; // extract
  assign n1323 = n1366[75]; // extract
  assign n1324 = n1366[79:76]; // extract
  assign n1325 = n1366[80]; // extract
  assign n1326 = n1366[81]; // extract
  assign n1327 = {\c_rsp_i_c_rsp_i[data] , \c_rsp_i_c_rsp_i[err] , \c_rsp_i_c_rsp_i[ack] };
  assign n1329 = n1367[4:0]; // extract
  assign n1330 = n1367[36:5]; // extract
  assign n1331 = n1367[68:37]; // extract
  assign n1332 = n1367[72:69]; // extract
  assign n1333 = n1367[73]; // extract
  assign n1334 = n1367[74]; // extract
  assign n1335 = n1367[75]; // extract
  assign n1336 = n1367[79:76]; // extract
  assign n1337 = n1367[80]; // extract
  assign n1338 = n1367[81]; // extract
  assign n1339 = {\x_rsp_i_x_rsp_i[data] , \x_rsp_i_x_rsp_i[err] , \x_rsp_i_x_rsp_i[ack] };
  /* ../../rtl/core/neorv32_bus.vhd:308:10  */
  assign port_sel = n1497; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:317:10  */
  assign port_req = n1498; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:318:10  */
  assign port_rsp = n1499; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:321:10  */
  assign int_rsp = n1416; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:334:10  */
  assign keeper = n1501; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:340:22  */
  assign n1342 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:341:22  */
  assign n1346 = 1'b0 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:342:47  */
  assign n1349 = n1287[36:26]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:342:66  */
  assign n1351 = n1349 == 11'b11111111111;
  /* ../../rtl/core/neorv32_bus.vhd:342:32  */
  assign n1353 = n1351 & 1'b1;
  /* ../../rtl/core/neorv32_bus.vhd:342:22  */
  assign n1354 = n1353 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:343:45  */
  assign n1357 = port_sel[2:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:343:58  */
  assign n1359 = n1357 == 3'b000;
  /* ../../rtl/core/neorv32_bus.vhd:343:32  */
  assign n1361 = n1359 & 1'b1;
  /* ../../rtl/core/neorv32_bus.vhd:343:22  */
  assign n1362 = n1361 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:347:22  */
  assign n1364 = port_req[327:246]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:348:22  */
  assign n1365 = port_req[245:164]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:349:22  */
  assign n1366 = port_req[163:82]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:350:22  */
  assign n1367 = port_req[81:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:359:34  */
  assign n1370 = n1287[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:359:50  */
  assign n1371 = port_sel[2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:359:38  */
  assign n1372 = n1370 & n1371;
  assign n1373 = n1287[81:74]; // extract
  assign n1374 = n1287[72:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:359:34  */
  assign n1375 = n1287[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:359:50  */
  assign n1376 = port_sel[3]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:359:38  */
  assign n1377 = n1375 & n1376;
  assign n1378 = n1287[81:74]; // extract
  assign n1379 = n1287[72:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:371:29  */
  assign n1384 = n1383[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:371:49  */
  assign n1386 = port_rsp[67:36]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:371:34  */
  assign n1387 = n1384 | n1386;
  assign n1389 = n1388[1:0]; // extract
  assign n1390 = {n1387, n1389};
  /* ../../rtl/core/neorv32_bus.vhd:372:29  */
  assign n1391 = n1390[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:372:49  */
  assign n1393 = port_rsp[34]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:372:34  */
  assign n1394 = n1391 | n1393;
  assign n1395 = n1388[1]; // extract
  assign n1396 = {n1387, n1395, n1394};
  /* ../../rtl/core/neorv32_bus.vhd:373:29  */
  assign n1397 = n1396[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:373:49  */
  assign n1399 = port_rsp[35]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:373:34  */
  assign n1400 = n1397 | n1399;
  assign n1401 = {n1387, n1400, n1394};
  /* ../../rtl/core/neorv32_bus.vhd:371:29  */
  assign n1402 = n1401[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:371:49  */
  assign n1404 = port_rsp[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:371:34  */
  assign n1405 = n1402 | n1404;
  assign n1406 = {n1405, n1400, n1394};
  /* ../../rtl/core/neorv32_bus.vhd:372:29  */
  assign n1407 = n1406[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:372:49  */
  assign n1409 = port_rsp[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:372:34  */
  assign n1410 = n1407 | n1409;
  assign n1411 = {n1405, n1400, n1410};
  /* ../../rtl/core/neorv32_bus.vhd:373:29  */
  assign n1412 = n1411[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:373:49  */
  assign n1414 = port_rsp[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:373:34  */
  assign n1415 = n1412 | n1414;
  assign n1416 = {n1405, n1415, n1410};
  /* ../../rtl/core/neorv32_bus.vhd:380:25  */
  assign n1419 = int_rsp[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:381:25  */
  assign n1420 = int_rsp[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:381:39  */
  assign n1421 = keeper[9]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:381:29  */
  assign n1422 = n1420 | n1421;
  /* ../../rtl/core/neorv32_bus.vhd:382:25  */
  assign n1423 = int_rsp[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:382:39  */
  assign n1424 = keeper[9]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:382:29  */
  assign n1425 = n1423 | n1424;
  /* ../../rtl/core/neorv32_bus.vhd:388:16  */
  assign n1427 = ~rstn_i;
  /* ../../rtl/core/neorv32_bus.vhd:394:19  */
  assign n1433 = keeper[1:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:398:32  */
  assign n1434 = n1287[81]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:399:34  */
  assign n1435 = port_sel[3]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:401:21  */
  assign n1437 = n1287[73]; // extract
  assign n1439 = keeper[1:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:401:11  */
  assign n1440 = n1437 ? 2'b01 : n1439;
  /* ../../rtl/core/neorv32_bus.vhd:396:9  */
  assign n1442 = n1433 == 2'b00;
  /* ../../rtl/core/neorv32_bus.vhd:408:23  */
  assign n1443 = int_rsp[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:411:61  */
  assign n1445 = keeper[8:4]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:411:66  */
  assign n1447 = n1445 + 5'b00001;
  /* ../../rtl/core/neorv32_bus.vhd:408:11  */
  assign n1448 = n1443 ? 5'b00000 : n1447;
  /* ../../rtl/core/neorv32_bus.vhd:414:23  */
  assign n1449 = keeper[3]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:414:27  */
  assign n1450 = ~n1449;
  /* ../../rtl/core/neorv32_bus.vhd:414:34  */
  assign n1452 = 1'b1 & n1450;
  /* ../../rtl/core/neorv32_bus.vhd:414:67  */
  assign n1453 = keeper[8]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:414:52  */
  assign n1454 = n1453 & n1452;
  /* ../../rtl/core/neorv32_bus.vhd:414:87  */
  assign n1457 = n1454 | 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:417:25  */
  assign n1459 = keeper[2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:418:23  */
  assign n1460 = n1287[81]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:418:28  */
  assign n1461 = ~n1460;
  assign n1463 = keeper[1:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:418:13  */
  assign n1464 = n1461 ? 2'b00 : n1463;
  /* ../../rtl/core/neorv32_bus.vhd:421:26  */
  assign n1465 = int_rsp[0]; // extract
  assign n1467 = keeper[1:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:421:11  */
  assign n1468 = n1465 ? 2'b00 : n1467;
  /* ../../rtl/core/neorv32_bus.vhd:417:11  */
  assign n1469 = n1459 ? n1464 : n1468;
  /* ../../rtl/core/neorv32_bus.vhd:414:11  */
  assign n1470 = n1457 ? 2'b11 : n1469;
  /* ../../rtl/core/neorv32_bus.vhd:405:9  */
  assign n1472 = n1433 == 2'b01;
  /* ../../rtl/core/neorv32_bus.vhd:427:22  */
  assign n1473 = keeper[2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:427:27  */
  assign n1474 = ~n1473;
  /* ../../rtl/core/neorv32_bus.vhd:427:44  */
  assign n1475 = n1287[81]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:427:49  */
  assign n1476 = ~n1475;
  /* ../../rtl/core/neorv32_bus.vhd:427:34  */
  assign n1477 = n1474 | n1476;
  assign n1479 = keeper[1:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:427:11  */
  assign n1480 = n1477 ? 2'b00 : n1479;
  assign n1481 = {n1472, n1442};
  /* ../../rtl/core/neorv32_bus.vhd:394:7  */
  always @*
    case (n1481)
      2'b10: n1482 = n1470;
      2'b01: n1482 = n1440;
      default: n1482 = n1480;
    endcase
  assign n1483 = keeper[2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:394:7  */
  always @*
    case (n1481)
      2'b10: n1484 = n1483;
      2'b01: n1484 = n1434;
      default: n1484 = n1483;
    endcase
  assign n1485 = keeper[3]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:394:7  */
  always @*
    case (n1481)
      2'b10: n1486 = n1485;
      2'b01: n1486 = n1435;
      default: n1486 = n1485;
    endcase
  assign n1487 = keeper[8:4]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:394:7  */
  always @*
    case (n1481)
      2'b10: n1488 = n1448;
      2'b01: n1488 = 5'b00000;
      default: n1488 = n1487;
    endcase
  assign n1489 = {n1488, n1486, n1484, n1482};
  assign n1492 = {5'b00000, 1'b0, 1'b0, 2'b00};
  /* ../../rtl/core/neorv32_bus.vhd:436:29  */
  assign n1495 = keeper[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:437:29  */
  assign n1496 = keeper[1]; // extract
  assign n1497 = {n1362, n1354, n1346, n1342};
  assign n1498 = {82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, 82'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000, n1373, n1372, n1374, n1378, n1377, n1379};
  assign n1499 = {n1303, n1315, n1327, n1339};
  /* ../../rtl/core/neorv32_bus.vhd:393:5  */
  always @(posedge clk_i or posedge n1427)
    if (n1427)
      n1500 <= n1492;
    else
      n1500 <= n1489;
  /* ../../rtl/core/neorv32_bus.vhd:388:5  */
  assign n1501 = {n1495, n1500};
  assign n1502 = {n1419, n1425, n1422};
endmodule

module neorv32_bus_switch_2547cc736e951fa4919853c43ae890861a3b3264
  (input  clk_i,
   input  rstn_i,
   input  [4:0] \a_req_i_a_req_i[meta] ,
   input  [31:0] \a_req_i_a_req_i[addr] ,
   input  [31:0] \a_req_i_a_req_i[data] ,
   input  [3:0] \a_req_i_a_req_i[ben] ,
   input  \a_req_i_a_req_i[stb] ,
   input  \a_req_i_a_req_i[rw] ,
   input  \a_req_i_a_req_i[amo] ,
   input  [3:0] \a_req_i_a_req_i[amoop] ,
   input  \a_req_i_a_req_i[burst] ,
   input  \a_req_i_a_req_i[lock] ,
   input  [4:0] \b_req_i_b_req_i[meta] ,
   input  [31:0] \b_req_i_b_req_i[addr] ,
   input  [31:0] \b_req_i_b_req_i[data] ,
   input  [3:0] \b_req_i_b_req_i[ben] ,
   input  \b_req_i_b_req_i[stb] ,
   input  \b_req_i_b_req_i[rw] ,
   input  \b_req_i_b_req_i[amo] ,
   input  [3:0] \b_req_i_b_req_i[amoop] ,
   input  \b_req_i_b_req_i[burst] ,
   input  \b_req_i_b_req_i[lock] ,
   input  \x_rsp_i_x_rsp_i[ack] ,
   input  \x_rsp_i_x_rsp_i[err] ,
   input  [31:0] \x_rsp_i_x_rsp_i[data] ,
   output \a_rsp_o_a_rsp_o[ack] ,
   output \a_rsp_o_a_rsp_o[err] ,
   output [31:0] \a_rsp_o_a_rsp_o[data] ,
   output \b_rsp_o_b_rsp_o[ack] ,
   output \b_rsp_o_b_rsp_o[err] ,
   output [31:0] \b_rsp_o_b_rsp_o[data] ,
   output [4:0] \x_req_o_x_req_o[meta] ,
   output [31:0] \x_req_o_x_req_o[addr] ,
   output [31:0] \x_req_o_x_req_o[data] ,
   output [3:0] \x_req_o_x_req_o[ben] ,
   output \x_req_o_x_req_o[stb] ,
   output \x_req_o_x_req_o[rw] ,
   output \x_req_o_x_req_o[amo] ,
   output [3:0] \x_req_o_x_req_o[amoop] ,
   output \x_req_o_x_req_o[burst] ,
   output \x_req_o_x_req_o[lock] );
  wire [81:0] n1105;
  wire n1107;
  wire n1108;
  wire [31:0] n1109;
  wire [81:0] n1110;
  wire n1112;
  wire n1113;
  wire [31:0] n1114;
  wire [4:0] n1116;
  wire [31:0] n1117;
  wire [31:0] n1118;
  wire [3:0] n1119;
  wire n1120;
  wire n1121;
  wire n1122;
  wire [3:0] n1123;
  wire n1124;
  wire n1125;
  wire [33:0] n1126;
  wire [1:0] state;
  wire [1:0] state_nxt;
  wire a_req;
  wire b_req;
  wire sel;
  wire sel_q;
  wire stb;
  wire [1:0] lock;
  wire [1:0] lock_nxt;
  wire n1128;
  wire n1131;
  wire n1132;
  wire n1134;
  wire n1136;
  wire n1138;
  wire n1139;
  wire n1141;
  wire n1143;
  wire n1161;
  wire n1162;
  wire n1163;
  wire n1164;
  wire n1165;
  wire n1166;
  wire n1167;
  wire n1168;
  wire n1169;
  wire n1170;
  wire [1:0] n1172;
  wire n1174;
  wire n1175;
  wire n1176;
  wire n1177;
  wire n1178;
  wire n1179;
  wire n1180;
  wire n1181;
  wire n1182;
  wire n1183;
  wire n1184;
  wire [1:0] n1186;
  wire n1188;
  wire n1189;
  wire n1190;
  wire [1:0] n1191;
  wire n1192;
  wire n1193;
  wire n1194;
  wire n1195;
  wire [1:0] n1197;
  wire n1200;
  wire n1203;
  wire [1:0] n1205;
  wire n1207;
  wire n1209;
  wire [1:0] n1210;
  reg [1:0] n1211;
  reg n1214;
  reg n1216;
  reg [1:0] n1218;
  wire [4:0] n1220;
  wire n1221;
  wire [4:0] n1222;
  wire [4:0] n1223;
  wire [31:0] n1224;
  wire n1225;
  wire [31:0] n1226;
  wire [31:0] n1227;
  wire [31:0] n1228;
  wire [31:0] n1230;
  wire [31:0] n1231;
  wire [31:0] n1233;
  wire [31:0] n1234;
  wire n1235;
  wire [31:0] n1236;
  wire [31:0] n1237;
  wire [3:0] n1238;
  wire n1239;
  wire [3:0] n1240;
  wire [3:0] n1241;
  wire n1242;
  wire n1243;
  wire n1244;
  wire n1245;
  wire n1246;
  wire n1247;
  wire n1248;
  wire n1249;
  wire [3:0] n1250;
  wire n1251;
  wire [3:0] n1252;
  wire [3:0] n1253;
  wire n1254;
  wire n1255;
  wire n1256;
  wire n1257;
  wire n1258;
  wire n1259;
  wire n1260;
  wire n1261;
  wire n1262;
  wire n1263;
  wire n1264;
  wire n1266;
  wire n1267;
  wire n1268;
  wire [31:0] n1270;
  wire n1271;
  wire n1272;
  wire n1274;
  wire n1275;
  wire [31:0] n1277;
  reg [1:0] n1278;
  reg n1279;
  reg n1280;
  reg n1281;
  reg [1:0] n1282;
  wire [33:0] n1283;
  wire [33:0] n1284;
  wire [81:0] n1285;
  assign \a_rsp_o_a_rsp_o[ack]  = n1107; //(module output)
  assign \a_rsp_o_a_rsp_o[err]  = n1108; //(module output)
  assign \a_rsp_o_a_rsp_o[data]  = n1109; //(module output)
  assign \b_rsp_o_b_rsp_o[ack]  = n1112; //(module output)
  assign \b_rsp_o_b_rsp_o[err]  = n1113; //(module output)
  assign \b_rsp_o_b_rsp_o[data]  = n1114; //(module output)
  assign \x_req_o_x_req_o[meta]  = n1116; //(module output)
  assign \x_req_o_x_req_o[addr]  = n1117; //(module output)
  assign \x_req_o_x_req_o[data]  = n1118; //(module output)
  assign \x_req_o_x_req_o[ben]  = n1119; //(module output)
  assign \x_req_o_x_req_o[stb]  = n1120; //(module output)
  assign \x_req_o_x_req_o[rw]  = n1121; //(module output)
  assign \x_req_o_x_req_o[amo]  = n1122; //(module output)
  assign \x_req_o_x_req_o[amoop]  = n1123; //(module output)
  assign \x_req_o_x_req_o[burst]  = n1124; //(module output)
  assign \x_req_o_x_req_o[lock]  = n1125; //(module output)
  assign n1105 = {\a_req_i_a_req_i[lock] , \a_req_i_a_req_i[burst] , \a_req_i_a_req_i[amoop] , \a_req_i_a_req_i[amo] , \a_req_i_a_req_i[rw] , \a_req_i_a_req_i[stb] , \a_req_i_a_req_i[ben] , \a_req_i_a_req_i[data] , \a_req_i_a_req_i[addr] , \a_req_i_a_req_i[meta] };
  assign n1107 = n1283[0]; // extract
  assign n1108 = n1283[1]; // extract
  assign n1109 = n1283[33:2]; // extract
  assign n1110 = {\b_req_i_b_req_i[lock] , \b_req_i_b_req_i[burst] , \b_req_i_b_req_i[amoop] , \b_req_i_b_req_i[amo] , \b_req_i_b_req_i[rw] , \b_req_i_b_req_i[stb] , \b_req_i_b_req_i[ben] , \b_req_i_b_req_i[data] , \b_req_i_b_req_i[addr] , \b_req_i_b_req_i[meta] };
  assign n1112 = n1284[0]; // extract
  assign n1113 = n1284[1]; // extract
  assign n1114 = n1284[33:2]; // extract
  assign n1116 = n1285[4:0]; // extract
  assign n1117 = n1285[36:5]; // extract
  assign n1118 = n1285[68:37]; // extract
  assign n1119 = n1285[72:69]; // extract
  assign n1120 = n1285[73]; // extract
  assign n1121 = n1285[74]; // extract
  assign n1122 = n1285[75]; // extract
  assign n1123 = n1285[79:76]; // extract
  assign n1124 = n1285[80]; // extract
  assign n1125 = n1285[81]; // extract
  /* ../../rtl/core/neorv32_cpu.vhd:232:19  */
  assign n1126 = {\x_rsp_i_x_rsp_i[data] , \x_rsp_i_x_rsp_i[err] , \x_rsp_i_x_rsp_i[ack] };
  /* ../../rtl/core/neorv32_bus.vhd:38:10  */
  assign state = n1278; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:38:17  */
  assign state_nxt = n1211; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:39:10  */
  assign a_req = n1279; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:39:17  */
  assign b_req = n1280; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:39:24  */
  assign sel = n1214; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:39:29  */
  assign sel_q = n1281; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:39:36  */
  assign stb = n1216; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:40:10  */
  assign lock = n1282; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:40:16  */
  assign lock_nxt = n1218; // (signal)
  /* ../../rtl/core/neorv32_bus.vhd:48:16  */
  assign n1128 = ~rstn_i;
  /* ../../rtl/core/neorv32_bus.vhd:58:17  */
  assign n1131 = state == 2'b01;
  /* ../../rtl/core/neorv32_bus.vhd:60:22  */
  assign n1132 = n1105[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:60:7  */
  assign n1134 = n1132 ? 1'b1 : a_req;
  /* ../../rtl/core/neorv32_bus.vhd:58:7  */
  assign n1136 = n1131 ? 1'b0 : n1134;
  /* ../../rtl/core/neorv32_bus.vhd:63:17  */
  assign n1138 = state == 2'b10;
  /* ../../rtl/core/neorv32_bus.vhd:65:22  */
  assign n1139 = n1110[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:65:7  */
  assign n1141 = n1139 ? 1'b1 : b_req;
  /* ../../rtl/core/neorv32_bus.vhd:63:7  */
  assign n1143 = n1138 ? 1'b0 : n1141;
  /* ../../rtl/core/neorv32_bus.vhd:87:24  */
  assign n1161 = n1105[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:88:18  */
  assign n1162 = lock[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:88:42  */
  assign n1163 = n1105[81]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:88:47  */
  assign n1164 = ~n1163;
  /* ../../rtl/core/neorv32_bus.vhd:88:29  */
  assign n1165 = n1164 & n1162;
  /* ../../rtl/core/neorv32_bus.vhd:88:64  */
  assign n1166 = lock[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:88:68  */
  assign n1167 = ~n1166;
  /* ../../rtl/core/neorv32_bus.vhd:88:88  */
  assign n1168 = n1126[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:88:75  */
  assign n1169 = n1168 & n1167;
  /* ../../rtl/core/neorv32_bus.vhd:88:55  */
  assign n1170 = n1165 | n1169;
  /* ../../rtl/core/neorv32_bus.vhd:88:9  */
  assign n1172 = n1170 ? 2'b00 : state;
  /* ../../rtl/core/neorv32_bus.vhd:84:7  */
  assign n1174 = state == 2'b01;
  /* ../../rtl/core/neorv32_bus.vhd:95:24  */
  assign n1175 = n1110[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:96:18  */
  assign n1176 = lock[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:96:42  */
  assign n1177 = n1110[81]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:96:47  */
  assign n1178 = ~n1177;
  /* ../../rtl/core/neorv32_bus.vhd:96:29  */
  assign n1179 = n1178 & n1176;
  /* ../../rtl/core/neorv32_bus.vhd:96:64  */
  assign n1180 = lock[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:96:68  */
  assign n1181 = ~n1180;
  /* ../../rtl/core/neorv32_bus.vhd:96:88  */
  assign n1182 = n1126[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:96:75  */
  assign n1183 = n1182 & n1181;
  /* ../../rtl/core/neorv32_bus.vhd:96:55  */
  assign n1184 = n1179 | n1183;
  /* ../../rtl/core/neorv32_bus.vhd:96:9  */
  assign n1186 = n1184 ? 2'b00 : state;
  /* ../../rtl/core/neorv32_bus.vhd:92:7  */
  assign n1188 = state == 2'b10;
  /* ../../rtl/core/neorv32_bus.vhd:102:29  */
  assign n1189 = n1110[81]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:102:44  */
  assign n1190 = n1105[81]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:102:34  */
  assign n1191 = {n1189, n1190};
  /* ../../rtl/core/neorv32_bus.vhd:104:23  */
  assign n1192 = n1105[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:104:34  */
  assign n1193 = n1192 | a_req;
  /* ../../rtl/core/neorv32_bus.vhd:108:26  */
  assign n1194 = n1110[73]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:108:37  */
  assign n1195 = n1194 | b_req;
  /* ../../rtl/core/neorv32_bus.vhd:108:11  */
  assign n1197 = n1195 ? 2'b10 : state;
  /* ../../rtl/core/neorv32_bus.vhd:108:11  */
  assign n1200 = n1195 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:108:11  */
  assign n1203 = n1195 ? 1'b1 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:104:11  */
  assign n1205 = n1193 ? 2'b01 : n1197;
  /* ../../rtl/core/neorv32_bus.vhd:104:11  */
  assign n1207 = n1193 ? 1'b0 : n1200;
  /* ../../rtl/core/neorv32_bus.vhd:104:11  */
  assign n1209 = n1193 ? 1'b1 : n1203;
  assign n1210 = {n1188, n1174};
  /* ../../rtl/core/neorv32_bus.vhd:82:5  */
  always @*
    case (n1210)
      2'b10: n1211 = n1186;
      2'b01: n1211 = n1172;
      default: n1211 = n1205;
    endcase
  /* ../../rtl/core/neorv32_bus.vhd:82:5  */
  always @*
    case (n1210)
      2'b10: n1214 = 1'b1;
      2'b01: n1214 = 1'b0;
      default: n1214 = n1207;
    endcase
  /* ../../rtl/core/neorv32_bus.vhd:82:5  */
  always @*
    case (n1210)
      2'b10: n1216 = n1175;
      2'b01: n1216 = n1161;
      default: n1216 = n1209;
    endcase
  /* ../../rtl/core/neorv32_bus.vhd:82:5  */
  always @*
    case (n1210)
      2'b10: n1218 = lock;
      2'b01: n1218 = lock;
      default: n1218 = n1191;
    endcase
  /* ../../rtl/core/neorv32_bus.vhd:130:28  */
  assign n1220 = n1105[4:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:130:44  */
  assign n1221 = ~sel;
  /* ../../rtl/core/neorv32_bus.vhd:130:34  */
  assign n1222 = n1221 ? n1220 : n1223;
  /* ../../rtl/core/neorv32_bus.vhd:130:64  */
  assign n1223 = n1110[4:0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:131:28  */
  assign n1224 = n1105[36:5]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:131:44  */
  assign n1225 = ~sel;
  /* ../../rtl/core/neorv32_bus.vhd:131:34  */
  assign n1226 = n1225 ? n1224 : n1227;
  /* ../../rtl/core/neorv32_bus.vhd:131:64  */
  assign n1227 = n1110[36:5]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:132:28  */
  assign n1228 = n1110[68:37]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:132:34  */
  assign n1230 = 1'b0 ? n1228 : n1233;
  /* ../../rtl/core/neorv32_bus.vhd:133:28  */
  assign n1231 = n1105[68:37]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:132:51  */
  assign n1233 = 1'b1 ? n1231 : n1236;
  /* ../../rtl/core/neorv32_bus.vhd:134:28  */
  assign n1234 = n1105[68:37]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:134:44  */
  assign n1235 = ~sel;
  /* ../../rtl/core/neorv32_bus.vhd:133:51  */
  assign n1236 = n1235 ? n1234 : n1237;
  /* ../../rtl/core/neorv32_bus.vhd:134:64  */
  assign n1237 = n1110[68:37]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:135:28  */
  assign n1238 = n1105[72:69]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:135:44  */
  assign n1239 = ~sel;
  /* ../../rtl/core/neorv32_bus.vhd:135:34  */
  assign n1240 = n1239 ? n1238 : n1241;
  /* ../../rtl/core/neorv32_bus.vhd:135:64  */
  assign n1241 = n1110[72:69]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:136:28  */
  assign n1242 = n1105[74]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:136:44  */
  assign n1243 = ~sel;
  /* ../../rtl/core/neorv32_bus.vhd:136:34  */
  assign n1244 = n1243 ? n1242 : n1245;
  /* ../../rtl/core/neorv32_bus.vhd:136:64  */
  assign n1245 = n1110[74]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:137:28  */
  assign n1246 = n1105[75]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:137:44  */
  assign n1247 = ~sel;
  /* ../../rtl/core/neorv32_bus.vhd:137:34  */
  assign n1248 = n1247 ? n1246 : n1249;
  /* ../../rtl/core/neorv32_bus.vhd:137:64  */
  assign n1249 = n1110[75]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:138:28  */
  assign n1250 = n1105[79:76]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:138:44  */
  assign n1251 = ~sel;
  /* ../../rtl/core/neorv32_bus.vhd:138:34  */
  assign n1252 = n1251 ? n1250 : n1253;
  /* ../../rtl/core/neorv32_bus.vhd:138:64  */
  assign n1253 = n1110[79:76]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:139:28  */
  assign n1254 = n1105[80]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:139:44  */
  assign n1255 = ~sel;
  /* ../../rtl/core/neorv32_bus.vhd:139:34  */
  assign n1256 = n1255 ? n1254 : n1257;
  /* ../../rtl/core/neorv32_bus.vhd:139:64  */
  assign n1257 = n1110[80]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:140:28  */
  assign n1258 = n1105[81]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:140:44  */
  assign n1259 = ~sel;
  /* ../../rtl/core/neorv32_bus.vhd:140:34  */
  assign n1260 = n1259 ? n1258 : n1261;
  /* ../../rtl/core/neorv32_bus.vhd:140:64  */
  assign n1261 = n1110[81]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:145:27  */
  assign n1262 = n1126[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:145:43  */
  assign n1263 = ~sel_q;
  /* ../../rtl/core/neorv32_bus.vhd:145:31  */
  assign n1264 = n1263 ? n1262 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:146:27  */
  assign n1266 = n1126[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:146:43  */
  assign n1267 = ~sel_q;
  /* ../../rtl/core/neorv32_bus.vhd:146:31  */
  assign n1268 = n1267 ? n1266 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:147:27  */
  assign n1270 = n1126[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:149:27  */
  assign n1271 = n1126[0]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:149:31  */
  assign n1272 = sel_q ? n1271 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:150:27  */
  assign n1274 = n1126[1]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:150:31  */
  assign n1275 = sel_q ? n1274 : 1'b0;
  /* ../../rtl/core/neorv32_bus.vhd:151:27  */
  assign n1277 = n1126[33:2]; // extract
  /* ../../rtl/core/neorv32_bus.vhd:54:5  */
  always @(posedge clk_i or posedge n1128)
    if (n1128)
      n1278 <= 2'b00;
    else
      n1278 <= state_nxt;
  /* ../../rtl/core/neorv32_bus.vhd:54:5  */
  always @(posedge clk_i or posedge n1128)
    if (n1128)
      n1279 <= 1'b0;
    else
      n1279 <= n1136;
  /* ../../rtl/core/neorv32_bus.vhd:54:5  */
  always @(posedge clk_i or posedge n1128)
    if (n1128)
      n1280 <= 1'b0;
    else
      n1280 <= n1143;
  /* ../../rtl/core/neorv32_bus.vhd:54:5  */
  always @(posedge clk_i or posedge n1128)
    if (n1128)
      n1281 <= 1'b0;
    else
      n1281 <= sel;
  /* ../../rtl/core/neorv32_bus.vhd:54:5  */
  always @(posedge clk_i or posedge n1128)
    if (n1128)
      n1282 <= 2'b00;
    else
      n1282 <= lock_nxt;
  /* ../../rtl/core/neorv32_bus.vhd:48:5  */
  assign n1283 = {n1270, n1268, n1264};
  /* ../../rtl/core/neorv32_bus.vhd:48:5  */
  assign n1284 = {n1277, n1275, n1272};
  /* ../../rtl/core/neorv32_bus.vhd:48:5  */
  assign n1285 = {n1260, n1256, n1252, n1248, n1244, stb, n1240, n1230, n1226, n1222};
endmodule

module neorv32_cpu_0_0_0_4_0_64_0_6c34b6f631e21924a9ad40faece5c46447e85d2d
  (input  clk_i,
   input  rstn_i,
   input  [63:0] mtime_i,
   input  msi_i,
   input  mei_i,
   input  mti_i,
   input  [15:0] firq_i,
   input  dbi_i,
   input  \ibus_rsp_i_ibus_rsp_i[ack] ,
   input  \ibus_rsp_i_ibus_rsp_i[err] ,
   input  [31:0] \ibus_rsp_i_ibus_rsp_i[data] ,
   input  \dbus_rsp_i_dbus_rsp_i[ack] ,
   input  \dbus_rsp_i_dbus_rsp_i[err] ,
   input  [31:0] \dbus_rsp_i_dbus_rsp_i[data] ,
   output \trace_o_trace_o[valid] ,
   output [31:0] \trace_o_trace_o[order] ,
   output [31:0] \trace_o_trace_o[insn] ,
   output \trace_o_trace_o[trap] ,
   output \trace_o_trace_o[halt] ,
   output \trace_o_trace_o[intr] ,
   output [1:0] \trace_o_trace_o[mode] ,
   output [1:0] \trace_o_trace_o[ixl] ,
   output \trace_o_trace_o[debug] ,
   output \trace_o_trace_o[compr] ,
   output \trace_o_trace_o[delta] ,
   output [31:0] \trace_o_trace_o[cmd32] ,
   output [4:0] \trace_o_trace_o[rs1_addr] ,
   output [4:0] \trace_o_trace_o[rs2_addr] ,
   output [31:0] \trace_o_trace_o[rs1_rdata] ,
   output [31:0] \trace_o_trace_o[rs2_rdata] ,
   output [4:0] \trace_o_trace_o[rd_addr] ,
   output [31:0] \trace_o_trace_o[rd_rdata] ,
   output [31:0] \trace_o_trace_o[pc_rdata] ,
   output [31:0] \trace_o_trace_o[pc_wdata] ,
   output [11:0] \trace_o_trace_o[csr_addr] ,
   output [31:0] \trace_o_trace_o[csr_rdata] ,
   output [31:0] \trace_o_trace_o[csr_wdata] ,
   output [31:0] \trace_o_trace_o[mem_addr] ,
   output [3:0] \trace_o_trace_o[mem_rmask] ,
   output [3:0] \trace_o_trace_o[mem_wmask] ,
   output [31:0] \trace_o_trace_o[mem_rdata] ,
   output [31:0] \trace_o_trace_o[mem_wdata] ,
   output sleep_o,
   output [1:0] fence_o,
   output [4:0] \ibus_req_o_ibus_req_o[meta] ,
   output [31:0] \ibus_req_o_ibus_req_o[addr] ,
   output [31:0] \ibus_req_o_ibus_req_o[data] ,
   output [3:0] \ibus_req_o_ibus_req_o[ben] ,
   output \ibus_req_o_ibus_req_o[stb] ,
   output \ibus_req_o_ibus_req_o[rw] ,
   output \ibus_req_o_ibus_req_o[amo] ,
   output [3:0] \ibus_req_o_ibus_req_o[amoop] ,
   output \ibus_req_o_ibus_req_o[burst] ,
   output \ibus_req_o_ibus_req_o[lock] ,
   output [4:0] \dbus_req_o_dbus_req_o[meta] ,
   output [31:0] \dbus_req_o_dbus_req_o[addr] ,
   output [31:0] \dbus_req_o_dbus_req_o[data] ,
   output [3:0] \dbus_req_o_dbus_req_o[ben] ,
   output \dbus_req_o_dbus_req_o[stb] ,
   output \dbus_req_o_dbus_req_o[rw] ,
   output \dbus_req_o_dbus_req_o[amo] ,
   output [3:0] \dbus_req_o_dbus_req_o[amoop] ,
   output \dbus_req_o_dbus_req_o[burst] ,
   output \dbus_req_o_dbus_req_o[lock] );
  wire n756;
  wire [31:0] n757;
  wire [31:0] n758;
  wire n759;
  wire n760;
  wire n761;
  wire [1:0] n762;
  wire [1:0] n763;
  wire n764;
  wire n765;
  wire n766;
  wire [31:0] n767;
  wire [4:0] n768;
  wire [4:0] n769;
  wire [31:0] n770;
  wire [31:0] n771;
  wire [4:0] n772;
  wire [31:0] n773;
  wire [31:0] n774;
  wire [31:0] n775;
  wire [11:0] n776;
  wire [31:0] n777;
  wire [31:0] n778;
  wire [31:0] n779;
  wire [3:0] n780;
  wire [3:0] n781;
  wire [31:0] n782;
  wire [31:0] n783;
  wire [4:0] n787;
  wire [31:0] n788;
  wire [31:0] n789;
  wire [3:0] n790;
  wire n791;
  wire n792;
  wire n793;
  wire [3:0] n794;
  wire n795;
  wire n796;
  wire [33:0] n797;
  wire [4:0] n799;
  wire [31:0] n800;
  wire [31:0] n801;
  wire [3:0] n802;
  wire n803;
  wire n804;
  wire n805;
  wire [3:0] n806;
  wire n807;
  wire n808;
  wire [33:0] n809;
  wire [263:0] ctrl;
  wire [50:0] frontend;
  wire [81:0] dbus_req;
  wire if_pmp_err;
  wire rw_pmp_err;
  wire hwtrig;
  wire [31:0] rf_wdata;
  wire [31:0] rs1;
  wire [31:0] rs2;
  wire [31:0] alu_res;
  wire [31:0] alu_add;
  wire [1:0] alu_cmp;
  wire alu_cp_done;
  wire [31:0] lsu_rdata;
  wire [31:0] lsu_mar;
  wire [3:0] lsu_err;
  wire lsu_wait;
  wire [31:0] csr_rdata;
  wire [2:0] irq_machine;
  wire [31:0] xcsr_tm;
  wire [31:0] xcsr_cnt;
  wire [31:0] xcsr_pmp;
  wire [31:0] xcsr_alu;
  wire [31:0] xcsr_res;
  wire [4:0] \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[meta] ;
  wire [31:0] \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[addr] ;
  wire [31:0] \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[data] ;
  wire [3:0] \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[ben] ;
  wire \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[stb] ;
  wire \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[rw] ;
  wire \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[amo] ;
  wire [3:0] \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[amoop] ;
  wire \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[burst] ;
  wire \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[lock] ;
  wire [31:0] \neorv32_cpu_frontend_inst.pmp_addr_o ;
  wire \neorv32_cpu_frontend_inst.pmp_priv_o ;
  wire \neorv32_cpu_frontend_inst.frontend_o_frontend_o[valid] ;
  wire [31:0] \neorv32_cpu_frontend_inst.frontend_o_frontend_o[i32] ;
  wire [15:0] \neorv32_cpu_frontend_inst.frontend_o_frontend_o[i16] ;
  wire \neorv32_cpu_frontend_inst.frontend_o_frontend_o[compr] ;
  wire \neorv32_cpu_frontend_inst.frontend_o_frontend_o[fault] ;
  wire n858;
  wire n859;
  wire [31:0] n860;
  wire [31:0] n861;
  wire [31:0] n862;
  wire n863;
  wire [4:0] n864;
  wire [4:0] n865;
  wire [4:0] n866;
  wire n867;
  wire [2:0] n868;
  wire n869;
  wire n870;
  wire n871;
  wire n872;
  wire [31:0] n873;
  wire n874;
  wire n875;
  wire n876;
  wire n877;
  wire n878;
  wire n879;
  wire n880;
  wire n881;
  wire n882;
  wire n883;
  wire n884;
  wire [11:0] n885;
  wire [31:0] n886;
  wire [10:0] n887;
  wire [2:0] n888;
  wire [11:0] n889;
  wire [6:0] n890;
  wire [15:0] n891;
  wire n892;
  wire n893;
  wire n894;
  wire n895;
  wire [1:0] n896;
  wire [81:0] n897;
  wire n899;
  wire n900;
  wire [31:0] n901;
  wire [50:0] n904;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[if_reset] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[if_ready] ;
  wire [31:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_cur] ;
  wire [31:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_nxt] ;
  wire [31:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_ret] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_wb_en] ;
  wire [4:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_rs1] ;
  wire [4:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_rs2] ;
  wire [4:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_rd] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_zero] ;
  wire [2:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_op] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_sub] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_opa_mux] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_opb_mux] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_unsigned] ;
  wire [31:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_imm] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_cp_alu] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_cp_cfu] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_cp_fpu] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_req] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_rd] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_wr] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_mo_en] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_mi_en] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_priv] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_we] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_re] ;
  wire [11:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_addr] ;
  wire [31:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_wdata] ;
  wire [10:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cnt_event] ;
  wire [2:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_funct3] ;
  wire [11:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_funct12] ;
  wire [6:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_opcode] ;
  wire [15:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_rvc] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_priv] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_trap] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_sync_exc] ;
  wire \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_debug] ;
  wire [1:0] \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_fence] ;
  wire [263:0] n906;
  wire n908;
  wire [31:0] n909;
  wire [15:0] n910;
  wire n911;
  wire n912;
  wire [1:0] n914;
  wire [2:0] n915;
  wire [31:0] n916;
  wire [31:0] n917;
  wire [31:0] n918;
  wire n919;
  wire n920;
  wire [1:0] n921;
  wire n924;
  wire n925;
  wire [31:0] n926;
  wire [31:0] n927;
  wire [31:0] n928;
  wire n929;
  wire [4:0] n930;
  wire [4:0] n931;
  wire [4:0] n932;
  wire n933;
  wire [2:0] n934;
  wire n935;
  wire n936;
  wire n937;
  wire n938;
  wire [31:0] n939;
  wire n940;
  wire n941;
  wire n942;
  wire n943;
  wire n944;
  wire n945;
  wire n946;
  wire n947;
  wire n948;
  wire n949;
  wire n950;
  wire [11:0] n951;
  wire [31:0] n952;
  wire [10:0] n953;
  wire [2:0] n954;
  wire [11:0] n955;
  wire [6:0] n956;
  wire [15:0] n957;
  wire n958;
  wire n959;
  wire n960;
  wire n961;
  wire [1:0] n962;
  wire n964;
  wire n965;
  wire [31:0] n966;
  wire [31:0] n967;
  wire [31:0] n968;
  wire n969;
  wire [4:0] n970;
  wire [4:0] n971;
  wire [4:0] n972;
  wire n973;
  wire [2:0] n974;
  wire n975;
  wire n976;
  wire n977;
  wire n978;
  wire [31:0] n979;
  wire n980;
  wire n981;
  wire n982;
  wire n983;
  wire n984;
  wire n985;
  wire n986;
  wire n987;
  wire n988;
  wire n989;
  wire n990;
  wire [11:0] n991;
  wire [31:0] n992;
  wire [10:0] n993;
  wire [2:0] n994;
  wire [11:0] n995;
  wire [6:0] n996;
  wire [15:0] n997;
  wire n998;
  wire n999;
  wire n1000;
  wire n1001;
  wire [1:0] n1002;
  wire [31:0] n1005;
  wire [31:0] n1006;
  wire [31:0] n1007;
  wire [31:0] n1008;
  wire n1009;
  wire n1010;
  wire [31:0] n1011;
  wire [31:0] n1012;
  wire [31:0] n1013;
  wire n1014;
  wire [4:0] n1015;
  wire [4:0] n1016;
  wire [4:0] n1017;
  wire n1018;
  wire [2:0] n1019;
  wire n1020;
  wire n1021;
  wire n1022;
  wire n1023;
  wire [31:0] n1024;
  wire n1025;
  wire n1026;
  wire n1027;
  wire n1028;
  wire n1029;
  wire n1030;
  wire n1031;
  wire n1032;
  wire n1033;
  wire n1034;
  wire n1035;
  wire [11:0] n1036;
  wire [31:0] n1037;
  wire [10:0] n1038;
  wire [2:0] n1039;
  wire [11:0] n1040;
  wire [6:0] n1041;
  wire [15:0] n1042;
  wire n1043;
  wire n1044;
  wire n1045;
  wire n1046;
  wire [1:0] n1047;
  wire [4:0] \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[meta] ;
  wire [31:0] \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[addr] ;
  wire [31:0] \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[data] ;
  wire [3:0] \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[ben] ;
  wire \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[stb] ;
  wire \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[rw] ;
  wire \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[amo] ;
  wire [3:0] \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[amoop] ;
  wire \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[burst] ;
  wire \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[lock] ;
  wire n1053;
  wire n1054;
  wire [31:0] n1055;
  wire [31:0] n1056;
  wire [31:0] n1057;
  wire n1058;
  wire [4:0] n1059;
  wire [4:0] n1060;
  wire [4:0] n1061;
  wire n1062;
  wire [2:0] n1063;
  wire n1064;
  wire n1065;
  wire n1066;
  wire n1067;
  wire [31:0] n1068;
  wire n1069;
  wire n1070;
  wire n1071;
  wire n1072;
  wire n1073;
  wire n1074;
  wire n1075;
  wire n1076;
  wire n1077;
  wire n1078;
  wire n1079;
  wire [11:0] n1080;
  wire [31:0] n1081;
  wire [10:0] n1082;
  wire [2:0] n1083;
  wire [11:0] n1084;
  wire [6:0] n1085;
  wire [15:0] n1086;
  wire n1087;
  wire n1088;
  wire n1089;
  wire n1090;
  wire [1:0] n1091;
  wire [81:0] n1096;
  wire n1098;
  wire n1099;
  wire [31:0] n1100;
  localparam [461:0] n1104 = 462'b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000;
  assign \trace_o_trace_o[valid]  = n756; //(module output)
  assign \trace_o_trace_o[order]  = n757; //(module output)
  assign \trace_o_trace_o[insn]  = n758; //(module output)
  assign \trace_o_trace_o[trap]  = n759; //(module output)
  assign \trace_o_trace_o[halt]  = n760; //(module output)
  assign \trace_o_trace_o[intr]  = n761; //(module output)
  assign \trace_o_trace_o[mode]  = n762; //(module output)
  assign \trace_o_trace_o[ixl]  = n763; //(module output)
  assign \trace_o_trace_o[debug]  = n764; //(module output)
  assign \trace_o_trace_o[compr]  = n765; //(module output)
  assign \trace_o_trace_o[delta]  = n766; //(module output)
  assign \trace_o_trace_o[cmd32]  = n767; //(module output)
  assign \trace_o_trace_o[rs1_addr]  = n768; //(module output)
  assign \trace_o_trace_o[rs2_addr]  = n769; //(module output)
  assign \trace_o_trace_o[rs1_rdata]  = n770; //(module output)
  assign \trace_o_trace_o[rs2_rdata]  = n771; //(module output)
  assign \trace_o_trace_o[rd_addr]  = n772; //(module output)
  assign \trace_o_trace_o[rd_rdata]  = n773; //(module output)
  assign \trace_o_trace_o[pc_rdata]  = n774; //(module output)
  assign \trace_o_trace_o[pc_wdata]  = n775; //(module output)
  assign \trace_o_trace_o[csr_addr]  = n776; //(module output)
  assign \trace_o_trace_o[csr_rdata]  = n777; //(module output)
  assign \trace_o_trace_o[csr_wdata]  = n778; //(module output)
  assign \trace_o_trace_o[mem_addr]  = n779; //(module output)
  assign \trace_o_trace_o[mem_rmask]  = n780; //(module output)
  assign \trace_o_trace_o[mem_wmask]  = n781; //(module output)
  assign \trace_o_trace_o[mem_rdata]  = n782; //(module output)
  assign \trace_o_trace_o[mem_wdata]  = n783; //(module output)
  assign sleep_o = n920; //(module output)
  assign fence_o = n921; //(module output)
  assign \ibus_req_o_ibus_req_o[meta]  = n787; //(module output)
  assign \ibus_req_o_ibus_req_o[addr]  = n788; //(module output)
  assign \ibus_req_o_ibus_req_o[data]  = n789; //(module output)
  assign \ibus_req_o_ibus_req_o[ben]  = n790; //(module output)
  assign \ibus_req_o_ibus_req_o[stb]  = n791; //(module output)
  assign \ibus_req_o_ibus_req_o[rw]  = n792; //(module output)
  assign \ibus_req_o_ibus_req_o[amo]  = n793; //(module output)
  assign \ibus_req_o_ibus_req_o[amoop]  = n794; //(module output)
  assign \ibus_req_o_ibus_req_o[burst]  = n795; //(module output)
  assign \ibus_req_o_ibus_req_o[lock]  = n796; //(module output)
  assign \dbus_req_o_dbus_req_o[meta]  = n799; //(module output)
  assign \dbus_req_o_dbus_req_o[addr]  = n800; //(module output)
  assign \dbus_req_o_dbus_req_o[data]  = n801; //(module output)
  assign \dbus_req_o_dbus_req_o[ben]  = n802; //(module output)
  assign \dbus_req_o_dbus_req_o[stb]  = n803; //(module output)
  assign \dbus_req_o_dbus_req_o[rw]  = n804; //(module output)
  assign \dbus_req_o_dbus_req_o[amo]  = n805; //(module output)
  assign \dbus_req_o_dbus_req_o[amoop]  = n806; //(module output)
  assign \dbus_req_o_dbus_req_o[burst]  = n807; //(module output)
  assign \dbus_req_o_dbus_req_o[lock]  = n808; //(module output)
  /* ../../rtl/core/neorv32_sys.vhd:113:3  */
  assign n756 = n1104[0]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:115:5  */
  assign n757 = n1104[32:1]; // extract
  assign n758 = n1104[64:33]; // extract
  assign n759 = n1104[65]; // extract
  assign n760 = n1104[66]; // extract
  assign n761 = n1104[67]; // extract
  assign n762 = n1104[69:68]; // extract
  assign n763 = n1104[71:70]; // extract
  assign n764 = n1104[72]; // extract
  assign n765 = n1104[73]; // extract
  assign n766 = n1104[74]; // extract
  assign n767 = n1104[106:75]; // extract
  assign n768 = n1104[111:107]; // extract
  /* ../../rtl/core/neorv32_top.vhd:1182:7  */
  assign n769 = n1104[116:112]; // extract
  assign n770 = n1104[148:117]; // extract
  assign n771 = n1104[180:149]; // extract
  assign n772 = n1104[185:181]; // extract
  assign n773 = n1104[217:186]; // extract
  assign n774 = n1104[249:218]; // extract
  assign n775 = n1104[281:250]; // extract
  assign n776 = n1104[293:282]; // extract
  assign n777 = n1104[325:294]; // extract
  assign n778 = n1104[357:326]; // extract
  assign n779 = n1104[389:358]; // extract
  assign n780 = n1104[393:390]; // extract
  /* ../../rtl/core/neorv32_top.vhd:855:16  */
  assign n781 = n1104[397:394]; // extract
  assign n782 = n1104[429:398]; // extract
  /* ../../rtl/core/neorv32_top.vhd:853:16  */
  assign n783 = n1104[461:430]; // extract
  /* ../../rtl/core/neorv32_top.vhd:590:21  */
  assign n787 = n897[4:0]; // extract
  /* ../../rtl/core/neorv32_top.vhd:492:19  */
  assign n788 = n897[36:5]; // extract
  /* ../../rtl/core/neorv32_top.vhd:480:22  */
  assign n789 = n897[68:37]; // extract
  /* ../../rtl/core/neorv32_top.vhd:346:30  */
  assign n790 = n897[72:69]; // extract
  /* ../../rtl/core/neorv32_top.vhd:345:68  */
  assign n791 = n897[73]; // extract
  /* ../../rtl/core/neorv32_top.vhd:345:58  */
  assign n792 = n897[74]; // extract
  /* ../../rtl/core/neorv32_top.vhd:345:30  */
  assign n793 = n897[75]; // extract
  /* ../../rtl/core/neorv32_top.vhd:336:10  */
  assign n794 = n897[79:76]; // extract
  /* ../../rtl/core/neorv32_top.vhd:324:10  */
  assign n795 = n897[80]; // extract
  /* ../../rtl/core/neorv32_top.vhd:323:10  */
  assign n796 = n897[81]; // extract
  /* ../../rtl/core/neorv32_top.vhd:320:10  */
  assign n797 = {\ibus_rsp_i_ibus_rsp_i[data] , \ibus_rsp_i_ibus_rsp_i[err] , \ibus_rsp_i_ibus_rsp_i[ack] };
  /* ../../rtl/core/neorv32_top.vhd:1184:9  */
  assign n799 = dbus_req[4:0]; // extract
  assign n800 = dbus_req[36:5]; // extract
  assign n801 = dbus_req[68:37]; // extract
  assign n802 = dbus_req[72:69]; // extract
  assign n803 = dbus_req[73]; // extract
  assign n804 = dbus_req[74]; // extract
  assign n805 = dbus_req[75]; // extract
  assign n806 = dbus_req[79:76]; // extract
  assign n807 = dbus_req[80]; // extract
  assign n808 = dbus_req[81]; // extract
  assign n809 = {\dbus_rsp_i_dbus_rsp_i[data] , \dbus_rsp_i_dbus_rsp_i[err] , \dbus_rsp_i_dbus_rsp_i[ack] };
  /* ../../rtl/core/neorv32_cpu.vhd:117:10  */
  assign ctrl = n906; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:118:10  */
  assign frontend = n904; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:119:10  */
  assign dbus_req = n1096; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:124:10  */
  assign if_pmp_err = 1'b0; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:125:10  */
  assign rw_pmp_err = 1'b0; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:126:10  */
  assign hwtrig = 1'b0; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:127:10  */
  assign rf_wdata = n1008; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:139:10  */
  assign irq_machine = n915; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:142:10  */
  assign xcsr_tm = 32'b00000000000000000000000000000000; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:142:29  */
  assign xcsr_pmp = 32'b00000000000000000000000000000000; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:142:49  */
  assign xcsr_res = n918; // (signal)
  /* ../../rtl/core/neorv32_cpu.vhd:216:3  */
  neorv32_cpu_frontend_0_0e356ba505631fbf715758bed27d503f8b260e3a neorv32_cpu_frontend_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\ctrl_i_ctrl_i[if_reset] (n858),
    .\ctrl_i_ctrl_i[if_ready] (n859),
    .\ctrl_i_ctrl_i[pc_cur] (n860),
    .\ctrl_i_ctrl_i[pc_nxt] (n861),
    .\ctrl_i_ctrl_i[pc_ret] (n862),
    .\ctrl_i_ctrl_i[rf_wb_en] (n863),
    .\ctrl_i_ctrl_i[rf_rs1] (n864),
    .\ctrl_i_ctrl_i[rf_rs2] (n865),
    .\ctrl_i_ctrl_i[rf_rd] (n866),
    .\ctrl_i_ctrl_i[rf_zero] (n867),
    .\ctrl_i_ctrl_i[alu_op] (n868),
    .\ctrl_i_ctrl_i[alu_sub] (n869),
    .\ctrl_i_ctrl_i[alu_opa_mux] (n870),
    .\ctrl_i_ctrl_i[alu_opb_mux] (n871),
    .\ctrl_i_ctrl_i[alu_unsigned] (n872),
    .\ctrl_i_ctrl_i[alu_imm] (n873),
    .\ctrl_i_ctrl_i[alu_cp_alu] (n874),
    .\ctrl_i_ctrl_i[alu_cp_cfu] (n875),
    .\ctrl_i_ctrl_i[alu_cp_fpu] (n876),
    .\ctrl_i_ctrl_i[lsu_req] (n877),
    .\ctrl_i_ctrl_i[lsu_rd] (n878),
    .\ctrl_i_ctrl_i[lsu_wr] (n879),
    .\ctrl_i_ctrl_i[lsu_mo_en] (n880),
    .\ctrl_i_ctrl_i[lsu_mi_en] (n881),
    .\ctrl_i_ctrl_i[lsu_priv] (n882),
    .\ctrl_i_ctrl_i[csr_we] (n883),
    .\ctrl_i_ctrl_i[csr_re] (n884),
    .\ctrl_i_ctrl_i[csr_addr] (n885),
    .\ctrl_i_ctrl_i[csr_wdata] (n886),
    .\ctrl_i_ctrl_i[cnt_event] (n887),
    .\ctrl_i_ctrl_i[ir_funct3] (n888),
    .\ctrl_i_ctrl_i[ir_funct12] (n889),
    .\ctrl_i_ctrl_i[ir_opcode] (n890),
    .\ctrl_i_ctrl_i[ir_rvc] (n891),
    .\ctrl_i_ctrl_i[cpu_priv] (n892),
    .\ctrl_i_ctrl_i[cpu_trap] (n893),
    .\ctrl_i_ctrl_i[cpu_sync_exc] (n894),
    .\ctrl_i_ctrl_i[cpu_debug] (n895),
    .\ctrl_i_ctrl_i[cpu_fence] (n896),
    .\ibus_rsp_i_ibus_rsp_i[ack] (n899),
    .\ibus_rsp_i_ibus_rsp_i[err] (n900),
    .\ibus_rsp_i_ibus_rsp_i[data] (n901),
    .pmp_err_i(if_pmp_err),
    .\ibus_req_o_ibus_req_o[meta] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[meta] ),
    .\ibus_req_o_ibus_req_o[addr] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[addr] ),
    .\ibus_req_o_ibus_req_o[data] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[data] ),
    .\ibus_req_o_ibus_req_o[ben] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[ben] ),
    .\ibus_req_o_ibus_req_o[stb] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[stb] ),
    .\ibus_req_o_ibus_req_o[rw] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[rw] ),
    .\ibus_req_o_ibus_req_o[amo] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[amo] ),
    .\ibus_req_o_ibus_req_o[amoop] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[amoop] ),
    .\ibus_req_o_ibus_req_o[burst] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[burst] ),
    .\ibus_req_o_ibus_req_o[lock] (\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[lock] ),
    .pmp_addr_o(),
    .pmp_priv_o(),
    .\frontend_o_frontend_o[valid] (\neorv32_cpu_frontend_inst.frontend_o_frontend_o[valid] ),
    .\frontend_o_frontend_o[i32] (\neorv32_cpu_frontend_inst.frontend_o_frontend_o[i32] ),
    .\frontend_o_frontend_o[i16] (\neorv32_cpu_frontend_inst.frontend_o_frontend_o[i16] ),
    .\frontend_o_frontend_o[compr] (\neorv32_cpu_frontend_inst.frontend_o_frontend_o[compr] ),
    .\frontend_o_frontend_o[fault] (\neorv32_cpu_frontend_inst.frontend_o_frontend_o[fault] ));
  assign n858 = ctrl[0]; // extract
  assign n859 = ctrl[1]; // extract
  assign n860 = ctrl[33:2]; // extract
  assign n861 = ctrl[65:34]; // extract
  assign n862 = ctrl[97:66]; // extract
  assign n863 = ctrl[98]; // extract
  assign n864 = ctrl[103:99]; // extract
  assign n865 = ctrl[108:104]; // extract
  assign n866 = ctrl[113:109]; // extract
  assign n867 = ctrl[114]; // extract
  assign n868 = ctrl[117:115]; // extract
  assign n869 = ctrl[118]; // extract
  assign n870 = ctrl[119]; // extract
  assign n871 = ctrl[120]; // extract
  assign n872 = ctrl[121]; // extract
  assign n873 = ctrl[153:122]; // extract
  assign n874 = ctrl[154]; // extract
  assign n875 = ctrl[155]; // extract
  assign n876 = ctrl[156]; // extract
  assign n877 = ctrl[157]; // extract
  assign n878 = ctrl[158]; // extract
  assign n879 = ctrl[159]; // extract
  assign n880 = ctrl[160]; // extract
  assign n881 = ctrl[161]; // extract
  assign n882 = ctrl[162]; // extract
  assign n883 = ctrl[163]; // extract
  assign n884 = ctrl[164]; // extract
  assign n885 = ctrl[176:165]; // extract
  assign n886 = ctrl[208:177]; // extract
  assign n887 = ctrl[219:209]; // extract
  assign n888 = ctrl[222:220]; // extract
  assign n889 = ctrl[234:223]; // extract
  assign n890 = ctrl[241:235]; // extract
  assign n891 = ctrl[257:242]; // extract
  assign n892 = ctrl[258]; // extract
  assign n893 = ctrl[259]; // extract
  assign n894 = ctrl[260]; // extract
  assign n895 = ctrl[261]; // extract
  assign n896 = ctrl[263:262]; // extract
  assign n897 = {\neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[lock] , \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[burst] , \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[amoop] , \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[amo] , \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[rw] , \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[stb] , \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[ben] , \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[data] , \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[addr] , \neorv32_cpu_frontend_inst.ibus_req_o_ibus_req_o[meta] };
  assign n899 = n797[0]; // extract
  assign n900 = n797[1]; // extract
  assign n901 = n797[33:2]; // extract
  assign n904 = {\neorv32_cpu_frontend_inst.frontend_o_frontend_o[fault] , \neorv32_cpu_frontend_inst.frontend_o_frontend_o[compr] , \neorv32_cpu_frontend_inst.frontend_o_frontend_o[i16] , \neorv32_cpu_frontend_inst.frontend_o_frontend_o[i32] , \neorv32_cpu_frontend_inst.frontend_o_frontend_o[valid] };
  /* ../../rtl/core/neorv32_cpu.vhd:241:3  */
  neorv32_cpu_control_0_e740762aedb36b110599737dc13103f0a38aaf3e neorv32_cpu_control_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\frontend_i_frontend_i[valid] (n908),
    .\frontend_i_frontend_i[i32] (n909),
    .\frontend_i_frontend_i[i16] (n910),
    .\frontend_i_frontend_i[compr] (n911),
    .\frontend_i_frontend_i[fault] (n912),
    .hwtrig_i(hwtrig),
    .alu_cp_done_i(alu_cp_done),
    .alu_cmp_i(alu_cmp),
    .alu_add_i(alu_add),
    .rf_rs1_i(rs1),
    .xcsr_rdata_i(xcsr_res),
    .irq_dbg_i(dbi_i),
    .irq_machine_i(irq_machine),
    .irq_fast_i(firq_i),
    .lsu_wait_i(lsu_wait),
    .lsu_mar_i(lsu_mar),
    .lsu_err_i(lsu_err),
    .\ctrl_o_ctrl_o[if_reset] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[if_reset] ),
    .\ctrl_o_ctrl_o[if_ready] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[if_ready] ),
    .\ctrl_o_ctrl_o[pc_cur] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_cur] ),
    .\ctrl_o_ctrl_o[pc_nxt] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_nxt] ),
    .\ctrl_o_ctrl_o[pc_ret] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_ret] ),
    .\ctrl_o_ctrl_o[rf_wb_en] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_wb_en] ),
    .\ctrl_o_ctrl_o[rf_rs1] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_rs1] ),
    .\ctrl_o_ctrl_o[rf_rs2] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_rs2] ),
    .\ctrl_o_ctrl_o[rf_rd] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_rd] ),
    .\ctrl_o_ctrl_o[rf_zero] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_zero] ),
    .\ctrl_o_ctrl_o[alu_op] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_op] ),
    .\ctrl_o_ctrl_o[alu_sub] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_sub] ),
    .\ctrl_o_ctrl_o[alu_opa_mux] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_opa_mux] ),
    .\ctrl_o_ctrl_o[alu_opb_mux] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_opb_mux] ),
    .\ctrl_o_ctrl_o[alu_unsigned] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_unsigned] ),
    .\ctrl_o_ctrl_o[alu_imm] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_imm] ),
    .\ctrl_o_ctrl_o[alu_cp_alu] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_cp_alu] ),
    .\ctrl_o_ctrl_o[alu_cp_cfu] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_cp_cfu] ),
    .\ctrl_o_ctrl_o[alu_cp_fpu] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_cp_fpu] ),
    .\ctrl_o_ctrl_o[lsu_req] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_req] ),
    .\ctrl_o_ctrl_o[lsu_rd] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_rd] ),
    .\ctrl_o_ctrl_o[lsu_wr] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_wr] ),
    .\ctrl_o_ctrl_o[lsu_mo_en] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_mo_en] ),
    .\ctrl_o_ctrl_o[lsu_mi_en] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_mi_en] ),
    .\ctrl_o_ctrl_o[lsu_priv] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_priv] ),
    .\ctrl_o_ctrl_o[csr_we] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_we] ),
    .\ctrl_o_ctrl_o[csr_re] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_re] ),
    .\ctrl_o_ctrl_o[csr_addr] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_addr] ),
    .\ctrl_o_ctrl_o[csr_wdata] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_wdata] ),
    .\ctrl_o_ctrl_o[cnt_event] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[cnt_event] ),
    .\ctrl_o_ctrl_o[ir_funct3] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_funct3] ),
    .\ctrl_o_ctrl_o[ir_funct12] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_funct12] ),
    .\ctrl_o_ctrl_o[ir_opcode] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_opcode] ),
    .\ctrl_o_ctrl_o[ir_rvc] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_rvc] ),
    .\ctrl_o_ctrl_o[cpu_priv] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_priv] ),
    .\ctrl_o_ctrl_o[cpu_trap] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_trap] ),
    .\ctrl_o_ctrl_o[cpu_sync_exc] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_sync_exc] ),
    .\ctrl_o_ctrl_o[cpu_debug] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_debug] ),
    .\ctrl_o_ctrl_o[cpu_fence] (\neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_fence] ),
    .csr_rdata_o(csr_rdata));
  assign n906 = {\neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_fence] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_debug] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_sync_exc] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_trap] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cpu_priv] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_rvc] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_opcode] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_funct12] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[ir_funct3] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[cnt_event] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_wdata] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_addr] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_re] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[csr_we] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_priv] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_mi_en] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_mo_en] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_wr] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_rd] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[lsu_req] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_cp_fpu] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_cp_cfu] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_cp_alu] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_imm] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_unsigned] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_opb_mux] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_opa_mux] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_sub] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[alu_op] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_zero] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_rd] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_rs2] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_rs1] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[rf_wb_en] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_ret] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_nxt] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[pc_cur] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[if_ready] , \neorv32_cpu_control_inst.ctrl_o_ctrl_o[if_reset] };
  assign n908 = frontend[0]; // extract
  assign n909 = frontend[32:1]; // extract
  assign n910 = frontend[48:33]; // extract
  assign n911 = frontend[49]; // extract
  assign n912 = frontend[50]; // extract
  /* ../../rtl/core/neorv32_cpu.vhd:315:24  */
  assign n914 = {mei_i, mti_i};
  /* ../../rtl/core/neorv32_cpu.vhd:315:32  */
  assign n915 = {n914, msi_i};
  /* ../../rtl/core/neorv32_cpu.vhd:318:23  */
  assign n916 = xcsr_tm | xcsr_cnt;
  /* ../../rtl/core/neorv32_cpu.vhd:318:35  */
  assign n917 = n916 | xcsr_alu;
  /* ../../rtl/core/neorv32_cpu.vhd:318:47  */
  assign n918 = n917 | xcsr_pmp;
  /* ../../rtl/core/neorv32_cpu.vhd:321:32  */
  assign n919 = ctrl[209]; // extract
  /* ../../rtl/core/neorv32_cpu.vhd:321:14  */
  assign n920 = ~n919;
  /* ../../rtl/core/neorv32_cpu.vhd:324:19  */
  assign n921 = ctrl[263:262]; // extract
  /* ../../rtl/core/neorv32_cpu.vhd:360:5  */
  neorv32_cpu_counters_0_64_3c585604e87f855973731fea83e21fab9392d2fc cnts_enabled_neorv32_cpu_counters_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\ctrl_i_ctrl_i[if_reset] (n924),
    .\ctrl_i_ctrl_i[if_ready] (n925),
    .\ctrl_i_ctrl_i[pc_cur] (n926),
    .\ctrl_i_ctrl_i[pc_nxt] (n927),
    .\ctrl_i_ctrl_i[pc_ret] (n928),
    .\ctrl_i_ctrl_i[rf_wb_en] (n929),
    .\ctrl_i_ctrl_i[rf_rs1] (n930),
    .\ctrl_i_ctrl_i[rf_rs2] (n931),
    .\ctrl_i_ctrl_i[rf_rd] (n932),
    .\ctrl_i_ctrl_i[rf_zero] (n933),
    .\ctrl_i_ctrl_i[alu_op] (n934),
    .\ctrl_i_ctrl_i[alu_sub] (n935),
    .\ctrl_i_ctrl_i[alu_opa_mux] (n936),
    .\ctrl_i_ctrl_i[alu_opb_mux] (n937),
    .\ctrl_i_ctrl_i[alu_unsigned] (n938),
    .\ctrl_i_ctrl_i[alu_imm] (n939),
    .\ctrl_i_ctrl_i[alu_cp_alu] (n940),
    .\ctrl_i_ctrl_i[alu_cp_cfu] (n941),
    .\ctrl_i_ctrl_i[alu_cp_fpu] (n942),
    .\ctrl_i_ctrl_i[lsu_req] (n943),
    .\ctrl_i_ctrl_i[lsu_rd] (n944),
    .\ctrl_i_ctrl_i[lsu_wr] (n945),
    .\ctrl_i_ctrl_i[lsu_mo_en] (n946),
    .\ctrl_i_ctrl_i[lsu_mi_en] (n947),
    .\ctrl_i_ctrl_i[lsu_priv] (n948),
    .\ctrl_i_ctrl_i[csr_we] (n949),
    .\ctrl_i_ctrl_i[csr_re] (n950),
    .\ctrl_i_ctrl_i[csr_addr] (n951),
    .\ctrl_i_ctrl_i[csr_wdata] (n952),
    .\ctrl_i_ctrl_i[cnt_event] (n953),
    .\ctrl_i_ctrl_i[ir_funct3] (n954),
    .\ctrl_i_ctrl_i[ir_funct12] (n955),
    .\ctrl_i_ctrl_i[ir_opcode] (n956),
    .\ctrl_i_ctrl_i[ir_rvc] (n957),
    .\ctrl_i_ctrl_i[cpu_priv] (n958),
    .\ctrl_i_ctrl_i[cpu_trap] (n959),
    .\ctrl_i_ctrl_i[cpu_sync_exc] (n960),
    .\ctrl_i_ctrl_i[cpu_debug] (n961),
    .\ctrl_i_ctrl_i[cpu_fence] (n962),
    .mtime_i(mtime_i),
    .rdata_o(xcsr_cnt));
  assign n924 = ctrl[0]; // extract
  assign n925 = ctrl[1]; // extract
  assign n926 = ctrl[33:2]; // extract
  assign n927 = ctrl[65:34]; // extract
  assign n928 = ctrl[97:66]; // extract
  assign n929 = ctrl[98]; // extract
  assign n930 = ctrl[103:99]; // extract
  assign n931 = ctrl[108:104]; // extract
  assign n932 = ctrl[113:109]; // extract
  assign n933 = ctrl[114]; // extract
  assign n934 = ctrl[117:115]; // extract
  assign n935 = ctrl[118]; // extract
  assign n936 = ctrl[119]; // extract
  assign n937 = ctrl[120]; // extract
  assign n938 = ctrl[121]; // extract
  assign n939 = ctrl[153:122]; // extract
  assign n940 = ctrl[154]; // extract
  assign n941 = ctrl[155]; // extract
  assign n942 = ctrl[156]; // extract
  assign n943 = ctrl[157]; // extract
  assign n944 = ctrl[158]; // extract
  assign n945 = ctrl[159]; // extract
  assign n946 = ctrl[160]; // extract
  assign n947 = ctrl[161]; // extract
  assign n948 = ctrl[162]; // extract
  assign n949 = ctrl[163]; // extract
  assign n950 = ctrl[164]; // extract
  assign n951 = ctrl[176:165]; // extract
  assign n952 = ctrl[208:177]; // extract
  assign n953 = ctrl[219:209]; // extract
  assign n954 = ctrl[222:220]; // extract
  assign n955 = ctrl[234:223]; // extract
  assign n956 = ctrl[241:235]; // extract
  assign n957 = ctrl[257:242]; // extract
  assign n958 = ctrl[258]; // extract
  assign n959 = ctrl[259]; // extract
  assign n960 = ctrl[260]; // extract
  assign n961 = ctrl[261]; // extract
  assign n962 = ctrl[263:262]; // extract
  /* ../../rtl/core/neorv32_cpu.vhd:389:3  */
  neorv32_cpu_regfile_32_5_0 neorv32_cpu_regfile_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\ctrl_i_ctrl_i[if_reset] (n964),
    .\ctrl_i_ctrl_i[if_ready] (n965),
    .\ctrl_i_ctrl_i[pc_cur] (n966),
    .\ctrl_i_ctrl_i[pc_nxt] (n967),
    .\ctrl_i_ctrl_i[pc_ret] (n968),
    .\ctrl_i_ctrl_i[rf_wb_en] (n969),
    .\ctrl_i_ctrl_i[rf_rs1] (n970),
    .\ctrl_i_ctrl_i[rf_rs2] (n971),
    .\ctrl_i_ctrl_i[rf_rd] (n972),
    .\ctrl_i_ctrl_i[rf_zero] (n973),
    .\ctrl_i_ctrl_i[alu_op] (n974),
    .\ctrl_i_ctrl_i[alu_sub] (n975),
    .\ctrl_i_ctrl_i[alu_opa_mux] (n976),
    .\ctrl_i_ctrl_i[alu_opb_mux] (n977),
    .\ctrl_i_ctrl_i[alu_unsigned] (n978),
    .\ctrl_i_ctrl_i[alu_imm] (n979),
    .\ctrl_i_ctrl_i[alu_cp_alu] (n980),
    .\ctrl_i_ctrl_i[alu_cp_cfu] (n981),
    .\ctrl_i_ctrl_i[alu_cp_fpu] (n982),
    .\ctrl_i_ctrl_i[lsu_req] (n983),
    .\ctrl_i_ctrl_i[lsu_rd] (n984),
    .\ctrl_i_ctrl_i[lsu_wr] (n985),
    .\ctrl_i_ctrl_i[lsu_mo_en] (n986),
    .\ctrl_i_ctrl_i[lsu_mi_en] (n987),
    .\ctrl_i_ctrl_i[lsu_priv] (n988),
    .\ctrl_i_ctrl_i[csr_we] (n989),
    .\ctrl_i_ctrl_i[csr_re] (n990),
    .\ctrl_i_ctrl_i[csr_addr] (n991),
    .\ctrl_i_ctrl_i[csr_wdata] (n992),
    .\ctrl_i_ctrl_i[cnt_event] (n993),
    .\ctrl_i_ctrl_i[ir_funct3] (n994),
    .\ctrl_i_ctrl_i[ir_funct12] (n995),
    .\ctrl_i_ctrl_i[ir_opcode] (n996),
    .\ctrl_i_ctrl_i[ir_rvc] (n997),
    .\ctrl_i_ctrl_i[cpu_priv] (n998),
    .\ctrl_i_ctrl_i[cpu_trap] (n999),
    .\ctrl_i_ctrl_i[cpu_sync_exc] (n1000),
    .\ctrl_i_ctrl_i[cpu_debug] (n1001),
    .\ctrl_i_ctrl_i[cpu_fence] (n1002),
    .rd_i(rf_wdata),
    .rs1_o(rs1),
    .rs2_o(rs2));
  assign n964 = ctrl[0]; // extract
  assign n965 = ctrl[1]; // extract
  assign n966 = ctrl[33:2]; // extract
  assign n967 = ctrl[65:34]; // extract
  assign n968 = ctrl[97:66]; // extract
  assign n969 = ctrl[98]; // extract
  assign n970 = ctrl[103:99]; // extract
  assign n971 = ctrl[108:104]; // extract
  assign n972 = ctrl[113:109]; // extract
  assign n973 = ctrl[114]; // extract
  assign n974 = ctrl[117:115]; // extract
  assign n975 = ctrl[118]; // extract
  assign n976 = ctrl[119]; // extract
  assign n977 = ctrl[120]; // extract
  assign n978 = ctrl[121]; // extract
  assign n979 = ctrl[153:122]; // extract
  assign n980 = ctrl[154]; // extract
  assign n981 = ctrl[155]; // extract
  assign n982 = ctrl[156]; // extract
  assign n983 = ctrl[157]; // extract
  assign n984 = ctrl[158]; // extract
  assign n985 = ctrl[159]; // extract
  assign n986 = ctrl[160]; // extract
  assign n987 = ctrl[161]; // extract
  assign n988 = ctrl[162]; // extract
  assign n989 = ctrl[163]; // extract
  assign n990 = ctrl[164]; // extract
  assign n991 = ctrl[176:165]; // extract
  assign n992 = ctrl[208:177]; // extract
  assign n993 = ctrl[219:209]; // extract
  assign n994 = ctrl[222:220]; // extract
  assign n995 = ctrl[234:223]; // extract
  assign n996 = ctrl[241:235]; // extract
  assign n997 = ctrl[257:242]; // extract
  assign n998 = ctrl[258]; // extract
  assign n999 = ctrl[259]; // extract
  assign n1000 = ctrl[260]; // extract
  assign n1001 = ctrl[261]; // extract
  assign n1002 = ctrl[263:262]; // extract
  /* ../../rtl/core/neorv32_cpu.vhd:407:23  */
  assign n1005 = alu_res | lsu_rdata;
  /* ../../rtl/core/neorv32_cpu.vhd:407:36  */
  assign n1006 = n1005 | csr_rdata;
  /* ../../rtl/core/neorv32_cpu.vhd:407:57  */
  assign n1007 = ctrl[97:66]; // extract
  /* ../../rtl/core/neorv32_cpu.vhd:407:49  */
  assign n1008 = n1006 | n1007;
  /* ../../rtl/core/neorv32_cpu.vhd:412:3  */
  neorv32_cpu_alu_9a68e0f891a604eadc414df454e914fb8b2693a9 neorv32_cpu_alu_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\ctrl_i_ctrl_i[if_reset] (n1009),
    .\ctrl_i_ctrl_i[if_ready] (n1010),
    .\ctrl_i_ctrl_i[pc_cur] (n1011),
    .\ctrl_i_ctrl_i[pc_nxt] (n1012),
    .\ctrl_i_ctrl_i[pc_ret] (n1013),
    .\ctrl_i_ctrl_i[rf_wb_en] (n1014),
    .\ctrl_i_ctrl_i[rf_rs1] (n1015),
    .\ctrl_i_ctrl_i[rf_rs2] (n1016),
    .\ctrl_i_ctrl_i[rf_rd] (n1017),
    .\ctrl_i_ctrl_i[rf_zero] (n1018),
    .\ctrl_i_ctrl_i[alu_op] (n1019),
    .\ctrl_i_ctrl_i[alu_sub] (n1020),
    .\ctrl_i_ctrl_i[alu_opa_mux] (n1021),
    .\ctrl_i_ctrl_i[alu_opb_mux] (n1022),
    .\ctrl_i_ctrl_i[alu_unsigned] (n1023),
    .\ctrl_i_ctrl_i[alu_imm] (n1024),
    .\ctrl_i_ctrl_i[alu_cp_alu] (n1025),
    .\ctrl_i_ctrl_i[alu_cp_cfu] (n1026),
    .\ctrl_i_ctrl_i[alu_cp_fpu] (n1027),
    .\ctrl_i_ctrl_i[lsu_req] (n1028),
    .\ctrl_i_ctrl_i[lsu_rd] (n1029),
    .\ctrl_i_ctrl_i[lsu_wr] (n1030),
    .\ctrl_i_ctrl_i[lsu_mo_en] (n1031),
    .\ctrl_i_ctrl_i[lsu_mi_en] (n1032),
    .\ctrl_i_ctrl_i[lsu_priv] (n1033),
    .\ctrl_i_ctrl_i[csr_we] (n1034),
    .\ctrl_i_ctrl_i[csr_re] (n1035),
    .\ctrl_i_ctrl_i[csr_addr] (n1036),
    .\ctrl_i_ctrl_i[csr_wdata] (n1037),
    .\ctrl_i_ctrl_i[cnt_event] (n1038),
    .\ctrl_i_ctrl_i[ir_funct3] (n1039),
    .\ctrl_i_ctrl_i[ir_funct12] (n1040),
    .\ctrl_i_ctrl_i[ir_opcode] (n1041),
    .\ctrl_i_ctrl_i[ir_rvc] (n1042),
    .\ctrl_i_ctrl_i[cpu_priv] (n1043),
    .\ctrl_i_ctrl_i[cpu_trap] (n1044),
    .\ctrl_i_ctrl_i[cpu_sync_exc] (n1045),
    .\ctrl_i_ctrl_i[cpu_debug] (n1046),
    .\ctrl_i_ctrl_i[cpu_fence] (n1047),
    .rs1_i(rs1),
    .rs2_i(rs2),
    .cmp_o(alu_cmp),
    .res_o(alu_res),
    .add_o(alu_add),
    .csr_o(xcsr_alu),
    .done_o(alu_cp_done));
  assign n1009 = ctrl[0]; // extract
  assign n1010 = ctrl[1]; // extract
  assign n1011 = ctrl[33:2]; // extract
  assign n1012 = ctrl[65:34]; // extract
  assign n1013 = ctrl[97:66]; // extract
  assign n1014 = ctrl[98]; // extract
  assign n1015 = ctrl[103:99]; // extract
  assign n1016 = ctrl[108:104]; // extract
  assign n1017 = ctrl[113:109]; // extract
  assign n1018 = ctrl[114]; // extract
  assign n1019 = ctrl[117:115]; // extract
  assign n1020 = ctrl[118]; // extract
  assign n1021 = ctrl[119]; // extract
  assign n1022 = ctrl[120]; // extract
  assign n1023 = ctrl[121]; // extract
  assign n1024 = ctrl[153:122]; // extract
  assign n1025 = ctrl[154]; // extract
  assign n1026 = ctrl[155]; // extract
  assign n1027 = ctrl[156]; // extract
  assign n1028 = ctrl[157]; // extract
  assign n1029 = ctrl[158]; // extract
  assign n1030 = ctrl[159]; // extract
  assign n1031 = ctrl[160]; // extract
  assign n1032 = ctrl[161]; // extract
  assign n1033 = ctrl[162]; // extract
  assign n1034 = ctrl[163]; // extract
  assign n1035 = ctrl[164]; // extract
  assign n1036 = ctrl[176:165]; // extract
  assign n1037 = ctrl[208:177]; // extract
  assign n1038 = ctrl[219:209]; // extract
  assign n1039 = ctrl[222:220]; // extract
  assign n1040 = ctrl[234:223]; // extract
  assign n1041 = ctrl[241:235]; // extract
  assign n1042 = ctrl[257:242]; // extract
  assign n1043 = ctrl[258]; // extract
  assign n1044 = ctrl[259]; // extract
  assign n1045 = ctrl[260]; // extract
  assign n1046 = ctrl[261]; // extract
  assign n1047 = ctrl[263:262]; // extract
  /* ../../rtl/core/neorv32_cpu.vhd:457:3  */
  neorv32_cpu_lsu_0_5ba93c9db0cff93f52b521d7420e43f6eda2784f neorv32_cpu_lsu_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .\ctrl_i_ctrl_i[if_reset] (n1053),
    .\ctrl_i_ctrl_i[if_ready] (n1054),
    .\ctrl_i_ctrl_i[pc_cur] (n1055),
    .\ctrl_i_ctrl_i[pc_nxt] (n1056),
    .\ctrl_i_ctrl_i[pc_ret] (n1057),
    .\ctrl_i_ctrl_i[rf_wb_en] (n1058),
    .\ctrl_i_ctrl_i[rf_rs1] (n1059),
    .\ctrl_i_ctrl_i[rf_rs2] (n1060),
    .\ctrl_i_ctrl_i[rf_rd] (n1061),
    .\ctrl_i_ctrl_i[rf_zero] (n1062),
    .\ctrl_i_ctrl_i[alu_op] (n1063),
    .\ctrl_i_ctrl_i[alu_sub] (n1064),
    .\ctrl_i_ctrl_i[alu_opa_mux] (n1065),
    .\ctrl_i_ctrl_i[alu_opb_mux] (n1066),
    .\ctrl_i_ctrl_i[alu_unsigned] (n1067),
    .\ctrl_i_ctrl_i[alu_imm] (n1068),
    .\ctrl_i_ctrl_i[alu_cp_alu] (n1069),
    .\ctrl_i_ctrl_i[alu_cp_cfu] (n1070),
    .\ctrl_i_ctrl_i[alu_cp_fpu] (n1071),
    .\ctrl_i_ctrl_i[lsu_req] (n1072),
    .\ctrl_i_ctrl_i[lsu_rd] (n1073),
    .\ctrl_i_ctrl_i[lsu_wr] (n1074),
    .\ctrl_i_ctrl_i[lsu_mo_en] (n1075),
    .\ctrl_i_ctrl_i[lsu_mi_en] (n1076),
    .\ctrl_i_ctrl_i[lsu_priv] (n1077),
    .\ctrl_i_ctrl_i[csr_we] (n1078),
    .\ctrl_i_ctrl_i[csr_re] (n1079),
    .\ctrl_i_ctrl_i[csr_addr] (n1080),
    .\ctrl_i_ctrl_i[csr_wdata] (n1081),
    .\ctrl_i_ctrl_i[cnt_event] (n1082),
    .\ctrl_i_ctrl_i[ir_funct3] (n1083),
    .\ctrl_i_ctrl_i[ir_funct12] (n1084),
    .\ctrl_i_ctrl_i[ir_opcode] (n1085),
    .\ctrl_i_ctrl_i[ir_rvc] (n1086),
    .\ctrl_i_ctrl_i[cpu_priv] (n1087),
    .\ctrl_i_ctrl_i[cpu_trap] (n1088),
    .\ctrl_i_ctrl_i[cpu_sync_exc] (n1089),
    .\ctrl_i_ctrl_i[cpu_debug] (n1090),
    .\ctrl_i_ctrl_i[cpu_fence] (n1091),
    .addr_i(alu_add),
    .wdata_i(rs2),
    .pmp_fault_i(rw_pmp_err),
    .\dbus_rsp_i_dbus_rsp_i[ack] (n1098),
    .\dbus_rsp_i_dbus_rsp_i[err] (n1099),
    .\dbus_rsp_i_dbus_rsp_i[data] (n1100),
    .rdata_o(lsu_rdata),
    .mar_o(lsu_mar),
    .wait_o(lsu_wait),
    .err_o(lsu_err),
    .\dbus_req_o_dbus_req_o[meta] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[meta] ),
    .\dbus_req_o_dbus_req_o[addr] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[addr] ),
    .\dbus_req_o_dbus_req_o[data] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[data] ),
    .\dbus_req_o_dbus_req_o[ben] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[ben] ),
    .\dbus_req_o_dbus_req_o[stb] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[stb] ),
    .\dbus_req_o_dbus_req_o[rw] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[rw] ),
    .\dbus_req_o_dbus_req_o[amo] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[amo] ),
    .\dbus_req_o_dbus_req_o[amoop] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[amoop] ),
    .\dbus_req_o_dbus_req_o[burst] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[burst] ),
    .\dbus_req_o_dbus_req_o[lock] (\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[lock] ));
  assign n1053 = ctrl[0]; // extract
  assign n1054 = ctrl[1]; // extract
  assign n1055 = ctrl[33:2]; // extract
  assign n1056 = ctrl[65:34]; // extract
  assign n1057 = ctrl[97:66]; // extract
  assign n1058 = ctrl[98]; // extract
  assign n1059 = ctrl[103:99]; // extract
  assign n1060 = ctrl[108:104]; // extract
  assign n1061 = ctrl[113:109]; // extract
  assign n1062 = ctrl[114]; // extract
  assign n1063 = ctrl[117:115]; // extract
  assign n1064 = ctrl[118]; // extract
  assign n1065 = ctrl[119]; // extract
  assign n1066 = ctrl[120]; // extract
  assign n1067 = ctrl[121]; // extract
  assign n1068 = ctrl[153:122]; // extract
  assign n1069 = ctrl[154]; // extract
  assign n1070 = ctrl[155]; // extract
  assign n1071 = ctrl[156]; // extract
  assign n1072 = ctrl[157]; // extract
  assign n1073 = ctrl[158]; // extract
  assign n1074 = ctrl[159]; // extract
  assign n1075 = ctrl[160]; // extract
  assign n1076 = ctrl[161]; // extract
  assign n1077 = ctrl[162]; // extract
  assign n1078 = ctrl[163]; // extract
  assign n1079 = ctrl[164]; // extract
  assign n1080 = ctrl[176:165]; // extract
  assign n1081 = ctrl[208:177]; // extract
  assign n1082 = ctrl[219:209]; // extract
  assign n1083 = ctrl[222:220]; // extract
  assign n1084 = ctrl[234:223]; // extract
  assign n1085 = ctrl[241:235]; // extract
  assign n1086 = ctrl[257:242]; // extract
  assign n1087 = ctrl[258]; // extract
  assign n1088 = ctrl[259]; // extract
  assign n1089 = ctrl[260]; // extract
  assign n1090 = ctrl[261]; // extract
  assign n1091 = ctrl[263:262]; // extract
  assign n1096 = {\neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[lock] , \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[burst] , \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[amoop] , \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[amo] , \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[rw] , \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[stb] , \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[ben] , \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[data] , \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[addr] , \neorv32_cpu_lsu_inst.dbus_req_o_dbus_req_o[meta] };
  assign n1098 = n809[0]; // extract
  assign n1099 = n809[1]; // extract
  assign n1100 = n809[33:2]; // extract
endmodule

module neorv32_sys_clock
  (input  clk_i,
   input  rstn_i,
   input  enable_i,
   output [7:0] clk_en_o);
  wire [11:0] cnt;
  wire [11:0] cnt2;
  wire [11:0] \edge ;
  wire n729;
  wire [11:0] n732;
  wire [11:0] n734;
  wire [11:0] n742;
  wire [11:0] n743;
  wire n744;
  wire n745;
  wire n746;
  wire n747;
  wire n748;
  wire n749;
  wire n750;
  wire n751;
  reg [11:0] n752;
  reg [11:0] n753;
  wire [7:0] n754;
  assign clk_en_o = n754; //(module output)
  /* ../../rtl/core/neorv32_sys.vhd:108:10  */
  assign cnt = n752; // (signal)
  /* ../../rtl/core/neorv32_sys.vhd:108:15  */
  assign cnt2 = n753; // (signal)
  /* ../../rtl/core/neorv32_sys.vhd:108:21  */
  assign \edge  = n743; // (signal)
  /* ../../rtl/core/neorv32_sys.vhd:115:16  */
  assign n729 = ~rstn_i;
  /* ../../rtl/core/neorv32_sys.vhd:120:48  */
  assign n732 = cnt + 12'b000000000001;
  /* ../../rtl/core/neorv32_sys.vhd:119:7  */
  assign n734 = enable_i ? n732 : 12'b000000000000;
  /* ../../rtl/core/neorv32_sys.vhd:129:20  */
  assign n742 = ~cnt2;
  /* ../../rtl/core/neorv32_sys.vhd:129:15  */
  assign n743 = cnt & n742;
  /* ../../rtl/core/neorv32_sys.vhd:132:34  */
  assign n744 = \edge [0]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:133:34  */
  assign n745 = \edge [1]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:134:34  */
  assign n746 = \edge [2]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:135:34  */
  assign n747 = \edge [5]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:136:34  */
  assign n748 = \edge [6]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:137:34  */
  assign n749 = \edge [9]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:138:34  */
  assign n750 = \edge [10]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:139:34  */
  assign n751 = \edge [11]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:118:5  */
  always @(posedge clk_i or posedge n729)
    if (n729)
      n752 <= 12'b000000000000;
    else
      n752 <= n734;
  /* ../../rtl/core/neorv32_sys.vhd:118:5  */
  always @(posedge clk_i or posedge n729)
    if (n729)
      n753 <= 12'b000000000000;
    else
      n753 <= cnt;
  /* ../../rtl/core/neorv32_sys.vhd:115:5  */
  assign n754 = {n751, n750, n749, n748, n747, n746, n745, n744};
endmodule

module neorv32_sys_reset
  (input  clk_i,
   input  rstn_ext_i,
   input  rstn_wdt_i,
   input  rstn_dbg_i,
   output rstn_ext_o,
   output rstn_sys_o,
   output xrstn_wdt_o,
   output xrstn_ocd_o);
  wire [3:0] sreg_ext;
  wire [3:0] sreg_sys;
  wire n653;
  wire [2:0] n655;
  wire [3:0] n657;
  wire n664;
  wire n666;
  wire n668;
  wire n669;
  wire n670;
  wire n671;
  wire n672;
  wire n673;
  wire n674;
  wire n675;
  wire n676;
  wire [2:0] n677;
  wire [3:0] n679;
  wire [3:0] n681;
  wire n688;
  wire n690;
  wire n692;
  wire n693;
  wire n694;
  wire n695;
  wire n696;
  wire n697;
  wire n712;
  reg [3:0] n721;
  reg [3:0] n722;
  reg n723;
  reg n724;
  reg n725;
  reg n726;
  assign rstn_ext_o = n723; //(module output)
  assign rstn_sys_o = n724; //(module output)
  assign xrstn_wdt_o = n725; //(module output)
  assign xrstn_ocd_o = n726; //(module output)
  /* ../../rtl/core/neorv32_sys.vhd:36:10  */
  assign sreg_ext = n721; // (signal)
  /* ../../rtl/core/neorv32_sys.vhd:36:20  */
  assign sreg_sys = n722; // (signal)
  /* ../../rtl/core/neorv32_sys.vhd:43:20  */
  assign n653 = ~rstn_ext_i;
  /* ../../rtl/core/neorv32_sys.vhd:50:29  */
  assign n655 = sreg_ext[2:0]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:50:56  */
  assign n657 = {n655, 1'b1};
  /* ../../rtl/core/neorv32_package.vhd:1232:19  */
  assign n664 = sreg_ext[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1232:14  */
  assign n666 = 1'b1 & n664;
  /* ../../rtl/core/neorv32_package.vhd:1232:19  */
  assign n668 = sreg_ext[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1232:14  */
  assign n669 = n666 & n668;
  /* ../../rtl/core/neorv32_package.vhd:1232:19  */
  assign n670 = sreg_ext[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1232:14  */
  assign n671 = n669 & n670;
  /* ../../rtl/core/neorv32_package.vhd:1232:19  */
  assign n672 = sreg_ext[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1232:14  */
  assign n673 = n671 & n672;
  /* ../../rtl/core/neorv32_sys.vhd:53:22  */
  assign n674 = ~rstn_wdt_i;
  /* ../../rtl/core/neorv32_sys.vhd:53:44  */
  assign n675 = ~rstn_dbg_i;
  /* ../../rtl/core/neorv32_sys.vhd:53:29  */
  assign n676 = n674 | n675;
  /* ../../rtl/core/neorv32_sys.vhd:56:29  */
  assign n677 = sreg_sys[2:0]; // extract
  /* ../../rtl/core/neorv32_sys.vhd:56:56  */
  assign n679 = {n677, 1'b1};
  /* ../../rtl/core/neorv32_sys.vhd:53:7  */
  assign n681 = n676 ? 4'b0000 : n679;
  /* ../../rtl/core/neorv32_package.vhd:1232:19  */
  assign n688 = sreg_sys[3]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1232:14  */
  assign n690 = 1'b1 & n688;
  /* ../../rtl/core/neorv32_package.vhd:1232:19  */
  assign n692 = sreg_sys[2]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1232:14  */
  assign n693 = n690 & n692;
  /* ../../rtl/core/neorv32_package.vhd:1232:19  */
  assign n694 = sreg_sys[1]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1232:14  */
  assign n695 = n693 & n694;
  /* ../../rtl/core/neorv32_package.vhd:1232:19  */
  assign n696 = sreg_sys[0]; // extract
  /* ../../rtl/core/neorv32_package.vhd:1232:14  */
  assign n697 = n695 & n696;
  /* ../../rtl/core/neorv32_sys.vhd:65:20  */
  assign n712 = ~rstn_ext_i;
  /* ../../rtl/core/neorv32_sys.vhd:48:5  */
  always @(posedge clk_i or posedge n653)
    if (n653)
      n721 <= 4'b0000;
    else
      n721 <= n657;
  /* ../../rtl/core/neorv32_sys.vhd:48:5  */
  always @(posedge clk_i or posedge n653)
    if (n653)
      n722 <= 4'b0000;
    else
      n722 <= n681;
  /* ../../rtl/core/neorv32_sys.vhd:48:5  */
  always @(posedge clk_i or posedge n653)
    if (n653)
      n723 <= 1'b0;
    else
      n723 <= n673;
  /* ../../rtl/core/neorv32_sys.vhd:48:5  */
  always @(posedge clk_i or posedge n653)
    if (n653)
      n724 <= 1'b0;
    else
      n724 <= n697;
  /* ../../rtl/core/neorv32_sys.vhd:68:5  */
  always @(posedge clk_i or posedge n712)
    if (n712)
      n725 <= 1'b0;
    else
      n725 <= rstn_wdt_i;
  /* ../../rtl/core/neorv32_sys.vhd:68:5  */
  always @(posedge clk_i or posedge n712)
    if (n712)
      n726 <= 1'b0;
    else
      n726 <= rstn_dbg_i;
endmodule

module neorv32_top_100000000_1_0_0_0_4_0_64_16384_8192_4_4_64_0_0_1_1_1_1_1_1_1_1_1_0_1_3_5_64_1_0_1_4_1_1_1_8434e1b66992c70a3dedd746c2310434f0b34b57
  (input  clk_i,
   input  rstn_i,
   input  jtag_tck_i,
   input  jtag_tdi_i,
   input  jtag_tms_i,
   input  [31:0] xbus_dat_i,
   input  xbus_ack_i,
   input  xbus_err_i,
   input  [31:0] slink_rx_dat_i,
   input  [3:0] slink_rx_src_i,
   input  slink_rx_val_i,
   input  slink_rx_lst_i,
   input  slink_tx_rdy_i,
   input  [31:0] gpio_i,
   input  uart0_rxd_i,
   input  uart0_ctsn_i,
   input  uart1_rxd_i,
   input  uart1_ctsn_i,
   input  spi_dat_i,
   input  sdi_clk_i,
   input  sdi_dat_i,
   input  sdi_csn_i,
   input  twi_sda_i,
   input  twi_scl_i,
   input  twd_sda_i,
   input  twd_scl_i,
   input  onewire_i,
   input  [255:0] cfs_in_i,
   input  irq_msi_i,
   input  irq_mti_i,
   input  irq_mei_i,
   output rstn_ocd_o,
   output rstn_wdt_o,
   output \trace_cpu0_o_trace_cpu0_o[valid] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[order] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[insn] ,
   output \trace_cpu0_o_trace_cpu0_o[trap] ,
   output \trace_cpu0_o_trace_cpu0_o[halt] ,
   output \trace_cpu0_o_trace_cpu0_o[intr] ,
   output [1:0] \trace_cpu0_o_trace_cpu0_o[mode] ,
   output [1:0] \trace_cpu0_o_trace_cpu0_o[ixl] ,
   output \trace_cpu0_o_trace_cpu0_o[debug] ,
   output \trace_cpu0_o_trace_cpu0_o[compr] ,
   output \trace_cpu0_o_trace_cpu0_o[delta] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[cmd32] ,
   output [4:0] \trace_cpu0_o_trace_cpu0_o[rs1_addr] ,
   output [4:0] \trace_cpu0_o_trace_cpu0_o[rs2_addr] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[rs1_rdata] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[rs2_rdata] ,
   output [4:0] \trace_cpu0_o_trace_cpu0_o[rd_addr] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[rd_rdata] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[pc_rdata] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[pc_wdata] ,
   output [11:0] \trace_cpu0_o_trace_cpu0_o[csr_addr] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[csr_rdata] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[csr_wdata] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[mem_addr] ,
   output [3:0] \trace_cpu0_o_trace_cpu0_o[mem_rmask] ,
   output [3:0] \trace_cpu0_o_trace_cpu0_o[mem_wmask] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[mem_rdata] ,
   output [31:0] \trace_cpu0_o_trace_cpu0_o[mem_wdata] ,
   output \trace_cpu1_o_trace_cpu1_o[valid] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[order] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[insn] ,
   output \trace_cpu1_o_trace_cpu1_o[trap] ,
   output \trace_cpu1_o_trace_cpu1_o[halt] ,
   output \trace_cpu1_o_trace_cpu1_o[intr] ,
   output [1:0] \trace_cpu1_o_trace_cpu1_o[mode] ,
   output [1:0] \trace_cpu1_o_trace_cpu1_o[ixl] ,
   output \trace_cpu1_o_trace_cpu1_o[debug] ,
   output \trace_cpu1_o_trace_cpu1_o[compr] ,
   output \trace_cpu1_o_trace_cpu1_o[delta] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[cmd32] ,
   output [4:0] \trace_cpu1_o_trace_cpu1_o[rs1_addr] ,
   output [4:0] \trace_cpu1_o_trace_cpu1_o[rs2_addr] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[rs1_rdata] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[rs2_rdata] ,
   output [4:0] \trace_cpu1_o_trace_cpu1_o[rd_addr] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[rd_rdata] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[pc_rdata] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[pc_wdata] ,
   output [11:0] \trace_cpu1_o_trace_cpu1_o[csr_addr] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[csr_rdata] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[csr_wdata] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[mem_addr] ,
   output [3:0] \trace_cpu1_o_trace_cpu1_o[mem_rmask] ,
   output [3:0] \trace_cpu1_o_trace_cpu1_o[mem_wmask] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[mem_rdata] ,
   output [31:0] \trace_cpu1_o_trace_cpu1_o[mem_wdata] ,
   output jtag_tdo_o,
   output [31:0] xbus_adr_o,
   output [31:0] xbus_dat_o,
   output [2:0] xbus_cti_o,
   output [2:0] xbus_tag_o,
   output xbus_we_o,
   output [3:0] xbus_sel_o,
   output xbus_stb_o,
   output xbus_cyc_o,
   output slink_rx_rdy_o,
   output [31:0] slink_tx_dat_o,
   output [3:0] slink_tx_dst_o,
   output slink_tx_val_o,
   output slink_tx_lst_o,
   output [31:0] gpio_dir_o,
   output [31:0] gpio_o,
   output uart0_txd_o,
   output uart0_rtsn_o,
   output uart1_txd_o,
   output uart1_rtsn_o,
   output spi_clk_o,
   output spi_dat_o,
   output [7:0] spi_csn_o,
   output sdi_dat_o,
   output twi_sda_o,
   output twi_scl_o,
   output twd_sda_o,
   output onewire_o,
   output [31:0] pwm_o,
   output [255:0] cfs_out_o,
   output neoled_o,
   output [63:0] mtime_time_o);
  wire n119;
  wire [31:0] n120;
  wire [31:0] n121;
  wire n122;
  wire n123;
  wire n124;
  wire [1:0] n125;
  wire [1:0] n126;
  wire n127;
  wire n128;
  wire n129;
  wire [31:0] n130;
  wire [4:0] n131;
  wire [4:0] n132;
  wire [31:0] n133;
  wire [31:0] n134;
  wire [4:0] n135;
  wire [31:0] n136;
  wire [31:0] n137;
  wire [31:0] n138;
  wire [11:0] n139;
  wire [31:0] n140;
  wire [31:0] n141;
  wire [31:0] n142;
  wire [3:0] n143;
  wire [3:0] n144;
  wire [31:0] n145;
  wire [31:0] n146;
  wire n148;
  wire [31:0] n149;
  wire [31:0] n150;
  wire n151;
  wire n152;
  wire n153;
  wire [1:0] n154;
  wire [1:0] n155;
  wire n156;
  wire n157;
  wire n158;
  wire [31:0] n159;
  wire [4:0] n160;
  wire [4:0] n161;
  wire [31:0] n162;
  wire [31:0] n163;
  wire [4:0] n164;
  wire [31:0] n165;
  wire [31:0] n166;
  wire [31:0] n167;
  wire [11:0] n168;
  wire [31:0] n169;
  wire [31:0] n170;
  wire [31:0] n171;
  wire [3:0] n172;
  wire [3:0] n173;
  wire [31:0] n174;
  wire [31:0] n175;
  wire rstn_wdt;
  wire rstn_sys;
  wire dci_ndmrstn;
  wire dci_haltreq;
  wire [461:0] cpu_trace;
  wire [81:0] cpu_i_req;
  wire [81:0] cpu_d_req;
  wire [81:0] icache_req;
  wire [81:0] dcache_req;
  wire [81:0] core_req;
  wire [33:0] cpu_i_rsp;
  wire [33:0] cpu_d_rsp;
  wire [33:0] icache_rsp;
  wire [33:0] dcache_rsp;
  wire [33:0] core_rsp;
  wire [81:0] sys1_req;
  wire [81:0] sys2_req;
  wire [81:0] amo_req;
  wire [81:0] sys3_req;
  wire [81:0] io_req;
  wire [81:0] xbus_req;
  wire [33:0] sys1_rsp;
  wire [33:0] sys2_rsp;
  wire [33:0] amo_rsp;
  wire [33:0] sys3_rsp;
  wire [33:0] imem_rsp;
  wire [33:0] dmem_rsp;
  wire [33:0] io_rsp;
  wire [33:0] xbus_rsp;
  wire xbus_terminate;
  wire [1721:0] iodev_req;
  wire [713:0] iodev_rsp;
  wire [14:0] firq;
  wire [15:0] cpu_firq;
  wire mti;
  wire msi;
  wire [63:0] mtime;
  wire [31:0] mtime_lo;
  wire \soc_generators_neorv32_sys_reset_inst.rstn_ext_o ;
  wire \soc_generators_neorv32_sys_reset_inst.xrstn_wdt_o ;
  wire \soc_generators_neorv32_sys_reset_inst.xrstn_ocd_o ;
  wire [7:0] \soc_generators_neorv32_sys_clock_inst.clk_en_o ;
  localparam n259 = 1'b1;
  wire n262;
  wire n263;
  wire n264;
  wire n265;
  wire n266;
  wire n267;
  wire n268;
  wire n269;
  wire n270;
  wire n271;
  wire n272;
  wire n273;
  wire n274;
  wire n275;
  wire n276;
  wire \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[valid] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[order] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[insn] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[trap] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[halt] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[intr] ;
  wire [1:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mode] ;
  wire [1:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[ixl] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[debug] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[compr] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[delta] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[cmd32] ;
  wire [4:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs1_addr] ;
  wire [4:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs2_addr] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs1_rdata] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs2_rdata] ;
  wire [4:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rd_addr] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rd_rdata] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[pc_rdata] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[pc_wdata] ;
  wire [11:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[csr_addr] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[csr_rdata] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[csr_wdata] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_addr] ;
  wire [3:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_rmask] ;
  wire [3:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_wmask] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_rdata] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_wdata] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.sleep_o ;
  wire [1:0] \core_complex_gen_n1_neorv32_cpu_inst.fence_o ;
  wire [4:0] \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[meta] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[addr] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[data] ;
  wire [3:0] \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[ben] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[stb] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[rw] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[amo] ;
  wire [3:0] \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[amoop] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[burst] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[lock] ;
  wire [4:0] \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[meta] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[addr] ;
  wire [31:0] \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[data] ;
  wire [3:0] \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[ben] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[stb] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[rw] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[amo] ;
  wire [3:0] \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[amoop] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[burst] ;
  wire \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[lock] ;
  wire [461:0] n277;
  wire [81:0] n280;
  wire n282;
  wire n283;
  wire [31:0] n284;
  wire [81:0] n285;
  wire n287;
  wire n288;
  wire [31:0] n289;
  wire \core_complex_gen_n1_neorv32_core_bus_switch_inst.a_rsp_o_a_rsp_o[ack] ;
  wire \core_complex_gen_n1_neorv32_core_bus_switch_inst.a_rsp_o_a_rsp_o[err] ;
  wire [31:0] \core_complex_gen_n1_neorv32_core_bus_switch_inst.a_rsp_o_a_rsp_o[data] ;
  wire \core_complex_gen_n1_neorv32_core_bus_switch_inst.b_rsp_o_b_rsp_o[ack] ;
  wire \core_complex_gen_n1_neorv32_core_bus_switch_inst.b_rsp_o_b_rsp_o[err] ;
  wire [31:0] \core_complex_gen_n1_neorv32_core_bus_switch_inst.b_rsp_o_b_rsp_o[data] ;
  wire [4:0] \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[meta] ;
  wire [31:0] \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[addr] ;
  wire [31:0] \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[data] ;
  wire [3:0] \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[ben] ;
  wire \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[stb] ;
  wire \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[rw] ;
  wire \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[amo] ;
  wire [3:0] \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[amoop] ;
  wire \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[burst] ;
  wire \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[lock] ;
  wire [4:0] n290;
  wire [31:0] n291;
  wire [31:0] n292;
  wire [3:0] n293;
  wire n294;
  wire n295;
  wire n296;
  wire [3:0] n297;
  wire n298;
  wire n299;
  wire [33:0] n300;
  wire [4:0] n302;
  wire [31:0] n303;
  wire [31:0] n304;
  wire [3:0] n305;
  wire n306;
  wire n307;
  wire n308;
  wire [3:0] n309;
  wire n310;
  wire n311;
  wire [33:0] n312;
  wire [81:0] n314;
  wire n316;
  wire n317;
  wire [31:0] n318;
  wire [461:0] n320;
  localparam [33:0] n322 = 34'b0000000000000000000000000000000000;
  wire \neorv32_bus_gateway_inst.rsp_o_rsp_o[ack] ;
  wire \neorv32_bus_gateway_inst.rsp_o_rsp_o[err] ;
  wire [31:0] \neorv32_bus_gateway_inst.rsp_o_rsp_o[data] ;
  wire [4:0] \neorv32_bus_gateway_inst.a_req_o_a_req_o[meta] ;
  wire [31:0] \neorv32_bus_gateway_inst.a_req_o_a_req_o[addr] ;
  wire [31:0] \neorv32_bus_gateway_inst.a_req_o_a_req_o[data] ;
  wire [3:0] \neorv32_bus_gateway_inst.a_req_o_a_req_o[ben] ;
  wire \neorv32_bus_gateway_inst.a_req_o_a_req_o[stb] ;
  wire \neorv32_bus_gateway_inst.a_req_o_a_req_o[rw] ;
  wire \neorv32_bus_gateway_inst.a_req_o_a_req_o[amo] ;
  wire [3:0] \neorv32_bus_gateway_inst.a_req_o_a_req_o[amoop] ;
  wire \neorv32_bus_gateway_inst.a_req_o_a_req_o[burst] ;
  wire \neorv32_bus_gateway_inst.a_req_o_a_req_o[lock] ;
  wire [4:0] \neorv32_bus_gateway_inst.b_req_o_b_req_o[meta] ;
  wire [31:0] \neorv32_bus_gateway_inst.b_req_o_b_req_o[addr] ;
  wire [31:0] \neorv32_bus_gateway_inst.b_req_o_b_req_o[data] ;
  wire [3:0] \neorv32_bus_gateway_inst.b_req_o_b_req_o[ben] ;
  wire \neorv32_bus_gateway_inst.b_req_o_b_req_o[stb] ;
  wire \neorv32_bus_gateway_inst.b_req_o_b_req_o[rw] ;
  wire \neorv32_bus_gateway_inst.b_req_o_b_req_o[amo] ;
  wire [3:0] \neorv32_bus_gateway_inst.b_req_o_b_req_o[amoop] ;
  wire \neorv32_bus_gateway_inst.b_req_o_b_req_o[burst] ;
  wire \neorv32_bus_gateway_inst.b_req_o_b_req_o[lock] ;
  wire [4:0] \neorv32_bus_gateway_inst.c_req_o_c_req_o[meta] ;
  wire [31:0] \neorv32_bus_gateway_inst.c_req_o_c_req_o[addr] ;
  wire [31:0] \neorv32_bus_gateway_inst.c_req_o_c_req_o[data] ;
  wire [3:0] \neorv32_bus_gateway_inst.c_req_o_c_req_o[ben] ;
  wire \neorv32_bus_gateway_inst.c_req_o_c_req_o[stb] ;
  wire \neorv32_bus_gateway_inst.c_req_o_c_req_o[rw] ;
  wire \neorv32_bus_gateway_inst.c_req_o_c_req_o[amo] ;
  wire [3:0] \neorv32_bus_gateway_inst.c_req_o_c_req_o[amoop] ;
  wire \neorv32_bus_gateway_inst.c_req_o_c_req_o[burst] ;
  wire \neorv32_bus_gateway_inst.c_req_o_c_req_o[lock] ;
  wire [4:0] \neorv32_bus_gateway_inst.x_req_o_x_req_o[meta] ;
  wire [31:0] \neorv32_bus_gateway_inst.x_req_o_x_req_o[addr] ;
  wire [31:0] \neorv32_bus_gateway_inst.x_req_o_x_req_o[data] ;
  wire [3:0] \neorv32_bus_gateway_inst.x_req_o_x_req_o[ben] ;
  wire \neorv32_bus_gateway_inst.x_req_o_x_req_o[stb] ;
  wire \neorv32_bus_gateway_inst.x_req_o_x_req_o[rw] ;
  wire \neorv32_bus_gateway_inst.x_req_o_x_req_o[amo] ;
  wire [3:0] \neorv32_bus_gateway_inst.x_req_o_x_req_o[amoop] ;
  wire \neorv32_bus_gateway_inst.x_req_o_x_req_o[burst] ;
  wire \neorv32_bus_gateway_inst.x_req_o_x_req_o[lock] ;
  wire [4:0] n327;
  wire [31:0] n328;
  wire [31:0] n329;
  wire [3:0] n330;
  wire n331;
  wire n332;
  wire n333;
  wire [3:0] n334;
  wire n335;
  wire n336;
  wire [33:0] n337;
  wire n341;
  wire n342;
  wire [31:0] n343;
  wire n346;
  wire n347;
  wire [31:0] n348;
  wire [81:0] n349;
  wire n351;
  wire n352;
  wire [31:0] n353;
  wire [81:0] n354;
  wire n356;
  wire n357;
  wire [31:0] n358;
  wire \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.bus_rsp_o_bus_rsp_o[ack] ;
  wire \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.bus_rsp_o_bus_rsp_o[err] ;
  wire [31:0] \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.bus_rsp_o_bus_rsp_o[data] ;
  wire [31:0] \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_adr_o ;
  wire [31:0] \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_dat_o ;
  wire [2:0] \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_cti_o ;
  wire [2:0] \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_tag_o ;
  wire \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_we_o ;
  wire [3:0] \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_sel_o ;
  wire \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_stb_o ;
  wire \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_cyc_o ;
  wire [4:0] n361;
  wire [31:0] n362;
  wire [31:0] n363;
  wire [3:0] n364;
  wire n365;
  wire n366;
  wire n367;
  wire [3:0] n368;
  wire n369;
  wire n370;
  wire [33:0] n371;
  wire \io_system_neorv32_bus_io_switch_inst.main_rsp_o_main_rsp_o[ack] ;
  wire \io_system_neorv32_bus_io_switch_inst.main_rsp_o_main_rsp_o[err] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.main_rsp_o_main_rsp_o[data] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_01_req_o_dev_01_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_02_req_o_dev_02_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_03_req_o_dev_03_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_04_req_o_dev_04_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_05_req_o_dev_05_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_06_req_o_dev_06_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_07_req_o_dev_07_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_08_req_o_dev_08_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_09_req_o_dev_09_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_14_req_o_dev_14_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_15_req_o_dev_15_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[lock] ;
  wire [4:0] \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[meta] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[addr] ;
  wire [31:0] \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[data] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[ben] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[stb] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[rw] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[amo] ;
  wire [3:0] \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[amoop] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[burst] ;
  wire \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[lock] ;
  wire [4:0] n381;
  wire [31:0] n382;
  wire [31:0] n383;
  wire [3:0] n384;
  wire n385;
  wire n386;
  wire n387;
  wire [3:0] n388;
  wire n389;
  wire n390;
  wire [33:0] n391;
  wire [81:0] n393;
  wire [33:0] n395;
  wire n396;
  wire n397;
  wire [31:0] n398;
  wire n400;
  wire n401;
  wire [31:0] n402;
  wire n404;
  wire n405;
  wire [31:0] n406;
  wire n408;
  wire n409;
  wire [31:0] n410;
  wire n412;
  wire n413;
  wire [31:0] n414;
  wire n416;
  wire n417;
  wire [31:0] n418;
  wire n420;
  wire n421;
  wire [31:0] n422;
  wire n424;
  wire n425;
  wire [31:0] n426;
  wire n428;
  wire n429;
  wire [31:0] n430;
  wire n432;
  wire n433;
  wire [31:0] n434;
  wire [81:0] n435;
  wire [33:0] n437;
  wire n438;
  wire n439;
  wire [31:0] n440;
  wire [81:0] n441;
  wire [33:0] n443;
  wire n444;
  wire n445;
  wire [31:0] n446;
  wire [81:0] n447;
  wire [33:0] n449;
  wire n450;
  wire n451;
  wire [31:0] n452;
  wire [81:0] n453;
  wire [33:0] n455;
  wire n456;
  wire n457;
  wire [31:0] n458;
  wire n460;
  wire n461;
  wire [31:0] n462;
  wire n464;
  wire n465;
  wire [31:0] n466;
  wire [81:0] n467;
  wire [33:0] n469;
  wire n470;
  wire n471;
  wire [31:0] n472;
  wire [81:0] n473;
  wire [33:0] n475;
  wire n476;
  wire n477;
  wire [31:0] n478;
  wire [81:0] n479;
  wire [33:0] n481;
  wire n482;
  wire n483;
  wire [31:0] n484;
  wire [81:0] n485;
  wire [33:0] n487;
  wire n488;
  wire n489;
  wire [31:0] n490;
  wire [81:0] n491;
  wire [33:0] n493;
  wire n494;
  wire n495;
  wire [31:0] n496;
  wire [81:0] n497;
  wire [33:0] n499;
  wire n500;
  wire n501;
  wire [31:0] n502;
  wire [81:0] n503;
  wire [33:0] n505;
  wire n506;
  wire n507;
  wire [31:0] n508;
  wire [81:0] n509;
  wire [33:0] n511;
  wire n512;
  wire n513;
  wire [31:0] n514;
  wire [81:0] n515;
  wire [33:0] n517;
  wire n518;
  wire n519;
  wire [31:0] n520;
  wire [81:0] n521;
  wire [33:0] n523;
  wire n524;
  wire n525;
  wire [31:0] n526;
  wire [81:0] n527;
  wire [33:0] n529;
  wire n530;
  wire n531;
  wire [31:0] n532;
  wire [81:0] n533;
  wire [33:0] n535;
  wire n536;
  wire n537;
  wire [31:0] n538;
  wire [81:0] n539;
  wire [33:0] n541;
  wire n542;
  wire n543;
  wire [31:0] n544;
  wire [81:0] n545;
  wire [33:0] n547;
  wire n548;
  wire n549;
  wire [31:0] n550;
  wire [81:0] n551;
  wire [33:0] n553;
  wire n554;
  wire n555;
  wire [31:0] n556;
  wire [81:0] n557;
  wire [33:0] n559;
  wire n560;
  wire n561;
  wire [31:0] n562;
  localparam [255:0] n564 = 256'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;
  localparam n565 = 1'b0;
  localparam [31:0] n567 = 32'b00000000000000000000000000000000;
  localparam [31:0] n568 = 32'b00000000000000000000000000000000;
  wire \io_system_neorv32_clint_enabled_neorv32_clint_inst.bus_rsp_o_bus_rsp_o[ack] ;
  wire \io_system_neorv32_clint_enabled_neorv32_clint_inst.bus_rsp_o_bus_rsp_o[err] ;
  wire [31:0] \io_system_neorv32_clint_enabled_neorv32_clint_inst.bus_rsp_o_bus_rsp_o[data] ;
  wire [81:0] n571;
  wire [4:0] n572;
  wire [31:0] n573;
  wire [31:0] n574;
  wire [3:0] n575;
  wire n576;
  wire n577;
  wire n578;
  wire [3:0] n579;
  wire n580;
  wire n581;
  wire [33:0] n582;
  wire n588;
  wire [31:0] n590;
  wire [31:0] n595;
  wire [63:0] n596;
  localparam n597 = 1'b0;
  localparam n598 = 1'b1;
  localparam n600 = 1'b0;
  localparam n601 = 1'b1;
  localparam n603 = 1'b0;
  localparam n604 = 1'b0;
  localparam [7:0] n605 = 8'b11111111;
  localparam n607 = 1'b1;
  localparam n608 = 1'b1;
  localparam n610 = 1'b1;
  localparam [31:0] n612 = 32'b00000000000000000000000000000000;
  localparam n615 = 1'b0;
  localparam n617 = 1'b1;
  localparam n620 = 1'b0;
  localparam [31:0] n621 = 32'b00000000000000000000000000000000;
  localparam [3:0] n622 = 4'b0000;
  localparam n623 = 1'b0;
  localparam n624 = 1'b0;
  wire \io_system_neorv32_sysinfo_inst.bus_rsp_o_bus_rsp_o[ack] ;
  wire \io_system_neorv32_sysinfo_inst.bus_rsp_o_bus_rsp_o[err] ;
  wire [31:0] \io_system_neorv32_sysinfo_inst.bus_rsp_o_bus_rsp_o[data] ;
  wire [81:0] n626;
  wire [4:0] n627;
  wire [31:0] n628;
  wire [31:0] n629;
  wire [3:0] n630;
  wire n631;
  wire n632;
  wire n633;
  wire [3:0] n634;
  wire n635;
  wire n636;
  wire [33:0] n637;
  wire [1721:0] n643;
  wire [713:0] n644;
  wire [14:0] n645;
  wire [15:0] n646;
  reg [31:0] n647;
  assign rstn_ocd_o = \soc_generators_neorv32_sys_reset_inst.xrstn_ocd_o ; //(module output)
  assign rstn_wdt_o = \soc_generators_neorv32_sys_reset_inst.xrstn_wdt_o ; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[valid]  = n119; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[order]  = n120; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[insn]  = n121; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[trap]  = n122; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[halt]  = n123; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[intr]  = n124; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[mode]  = n125; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[ixl]  = n126; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[debug]  = n127; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[compr]  = n128; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[delta]  = n129; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[cmd32]  = n130; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[rs1_addr]  = n131; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[rs2_addr]  = n132; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[rs1_rdata]  = n133; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[rs2_rdata]  = n134; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[rd_addr]  = n135; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[rd_rdata]  = n136; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[pc_rdata]  = n137; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[pc_wdata]  = n138; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[csr_addr]  = n139; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[csr_rdata]  = n140; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[csr_wdata]  = n141; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[mem_addr]  = n142; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[mem_rmask]  = n143; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[mem_wmask]  = n144; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[mem_rdata]  = n145; //(module output)
  assign \trace_cpu0_o_trace_cpu0_o[mem_wdata]  = n146; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[valid]  = n148; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[order]  = n149; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[insn]  = n150; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[trap]  = n151; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[halt]  = n152; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[intr]  = n153; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[mode]  = n154; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[ixl]  = n155; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[debug]  = n156; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[compr]  = n157; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[delta]  = n158; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[cmd32]  = n159; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[rs1_addr]  = n160; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[rs2_addr]  = n161; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[rs1_rdata]  = n162; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[rs2_rdata]  = n163; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[rd_addr]  = n164; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[rd_rdata]  = n165; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[pc_rdata]  = n166; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[pc_wdata]  = n167; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[csr_addr]  = n168; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[csr_rdata]  = n169; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[csr_wdata]  = n170; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[mem_addr]  = n171; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[mem_rmask]  = n172; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[mem_wmask]  = n173; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[mem_rdata]  = n174; //(module output)
  assign \trace_cpu1_o_trace_cpu1_o[mem_wdata]  = n175; //(module output)
  assign jtag_tdo_o = jtag_tdi_i; //(module output)
  assign xbus_adr_o = \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_adr_o ; //(module output)
  assign xbus_dat_o = \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_dat_o ; //(module output)
  assign xbus_cti_o = \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_cti_o ; //(module output)
  assign xbus_tag_o = \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_tag_o ; //(module output)
  assign xbus_we_o = \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_we_o ; //(module output)
  assign xbus_sel_o = \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_sel_o ; //(module output)
  assign xbus_stb_o = \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_stb_o ; //(module output)
  assign xbus_cyc_o = \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_cyc_o ; //(module output)
  assign slink_rx_rdy_o = n620; //(module output)
  assign slink_tx_dat_o = n621; //(module output)
  assign slink_tx_dst_o = n622; //(module output)
  assign slink_tx_val_o = n623; //(module output)
  assign slink_tx_lst_o = n624; //(module output)
  assign gpio_dir_o = n567; //(module output)
  assign gpio_o = n568; //(module output)
  assign uart0_txd_o = n597; //(module output)
  assign uart0_rtsn_o = n598; //(module output)
  assign uart1_txd_o = n600; //(module output)
  assign uart1_rtsn_o = n601; //(module output)
  assign spi_clk_o = n603; //(module output)
  assign spi_dat_o = n604; //(module output)
  assign spi_csn_o = n605; //(module output)
  assign sdi_dat_o = n565; //(module output)
  assign twi_sda_o = n607; //(module output)
  assign twi_scl_o = n608; //(module output)
  assign twd_sda_o = n610; //(module output)
  assign onewire_o = n617; //(module output)
  assign pwm_o = n612; //(module output)
  assign cfs_out_o = n564; //(module output)
  assign neoled_o = n615; //(module output)
  assign mtime_time_o = n596; //(module output)
  assign n119 = cpu_trace[0]; // extract
  assign n120 = cpu_trace[32:1]; // extract
  assign n121 = cpu_trace[64:33]; // extract
  assign n122 = cpu_trace[65]; // extract
  assign n123 = cpu_trace[66]; // extract
  assign n124 = cpu_trace[67]; // extract
  assign n125 = cpu_trace[69:68]; // extract
  assign n126 = cpu_trace[71:70]; // extract
  assign n127 = cpu_trace[72]; // extract
  assign n128 = cpu_trace[73]; // extract
  assign n129 = cpu_trace[74]; // extract
  assign n130 = cpu_trace[106:75]; // extract
  assign n131 = cpu_trace[111:107]; // extract
  assign n132 = cpu_trace[116:112]; // extract
  assign n133 = cpu_trace[148:117]; // extract
  assign n134 = cpu_trace[180:149]; // extract
  assign n135 = cpu_trace[185:181]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n136 = cpu_trace[217:186]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n137 = cpu_trace[249:218]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n138 = cpu_trace[281:250]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n139 = cpu_trace[293:282]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n140 = cpu_trace[325:294]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n141 = cpu_trace[357:326]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n142 = cpu_trace[389:358]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n143 = cpu_trace[393:390]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n144 = cpu_trace[397:394]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n145 = cpu_trace[429:398]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n146 = cpu_trace[461:430]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n148 = n320[0]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n149 = n320[32:1]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n150 = n320[64:33]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n151 = n320[65]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n152 = n320[66]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n153 = n320[67]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n154 = n320[69:68]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n155 = n320[71:70]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n156 = n320[72]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n157 = n320[73]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n158 = n320[74]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n159 = n320[106:75]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n160 = n320[111:107]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n161 = n320[116:112]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n162 = n320[148:117]; // extract
  assign n163 = n320[180:149]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n164 = n320[185:181]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n165 = n320[217:186]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n166 = n320[249:218]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n167 = n320[281:250]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n168 = n320[293:282]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n169 = n320[325:294]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n170 = n320[357:326]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n171 = n320[389:358]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n172 = n320[393:390]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n173 = n320[397:394]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n174 = n320[429:398]; // extract
  /* neorv32_minimal_wrapper.vhd:47:3  */
  assign n175 = n320[461:430]; // extract
  /* ../../rtl/core/neorv32_top.vhd:317:10  */
  assign rstn_wdt = 1'b1; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:327:10  */
  assign dci_ndmrstn = 1'b1; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:328:10  */
  assign dci_haltreq = 1'b0; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:332:10  */
  assign cpu_trace = n277; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:341:10  */
  assign cpu_i_req = n280; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:341:21  */
  assign cpu_d_req = n285; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:341:32  */
  assign icache_req = cpu_i_req; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:341:44  */
  assign dcache_req = cpu_d_req; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:341:56  */
  assign core_req = n314; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:342:10  */
  assign cpu_i_rsp = icache_rsp; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:342:21  */
  assign cpu_d_rsp = dcache_rsp; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:342:32  */
  assign icache_rsp = n312; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:342:44  */
  assign dcache_rsp = n300; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:342:56  */
  assign core_rsp = sys1_rsp; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:345:10  */
  assign sys1_req = core_req; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:345:20  */
  assign sys2_req = sys1_req; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:345:39  */
  assign amo_req = sys2_req; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:345:48  */
  assign sys3_req = amo_req; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:345:78  */
  assign io_req = n349; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:345:86  */
  assign xbus_req = n354; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:346:10  */
  assign sys1_rsp = sys2_rsp; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:346:20  */
  assign sys2_rsp = amo_rsp; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:346:39  */
  assign amo_rsp = sys3_rsp; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:346:48  */
  assign sys3_rsp = n337; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:346:58  */
  assign imem_rsp = 34'b0000000000000000000000000000000000; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:346:68  */
  assign dmem_rsp = 34'b0000000000000000000000000000000000; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:346:78  */
  assign io_rsp = n391; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:346:86  */
  assign xbus_rsp = n371; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:357:10  */
  assign iodev_req = n643; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:358:10  */
  assign iodev_rsp = n644; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:366:10  */
  assign firq = n645; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:367:10  */
  assign cpu_firq = n646; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:372:10  */
  assign mtime_lo = n647; // (signal)
  /* ../../rtl/core/neorv32_top.vhd:474:5  */
  neorv32_sys_reset soc_generators_neorv32_sys_reset_inst (
    .clk_i(clk_i),
    .rstn_ext_i(rstn_i),
    .rstn_wdt_i(rstn_wdt),
    .rstn_dbg_i(dci_ndmrstn),
    .rstn_ext_o(),
    .rstn_sys_o(rstn_sys),
    .xrstn_wdt_o(\soc_generators_neorv32_sys_reset_inst.xrstn_wdt_o ),
    .xrstn_ocd_o(\soc_generators_neorv32_sys_reset_inst.xrstn_ocd_o ));
  /* ../../rtl/core/neorv32_top.vhd:487:5  */
  neorv32_sys_clock soc_generators_neorv32_sys_clock_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_sys),
    .enable_i(n259),
    .clk_en_o());
  /* ../../rtl/core/neorv32_top.vhd:503:23  */
  assign n262 = firq[8]; // extract
  /* ../../rtl/core/neorv32_top.vhd:504:23  */
  assign n263 = firq[13]; // extract
  /* ../../rtl/core/neorv32_top.vhd:505:23  */
  assign n264 = firq[12]; // extract
  /* ../../rtl/core/neorv32_top.vhd:506:23  */
  assign n265 = firq[14]; // extract
  /* ../../rtl/core/neorv32_top.vhd:507:23  */
  assign n266 = firq[0]; // extract
  /* ../../rtl/core/neorv32_top.vhd:508:23  */
  assign n267 = firq[11]; // extract
  /* ../../rtl/core/neorv32_top.vhd:509:23  */
  assign n268 = firq[9]; // extract
  /* ../../rtl/core/neorv32_top.vhd:510:23  */
  assign n269 = firq[6]; // extract
  /* ../../rtl/core/neorv32_top.vhd:511:23  */
  assign n270 = firq[7]; // extract
  /* ../../rtl/core/neorv32_top.vhd:512:23  */
  assign n271 = firq[3]; // extract
  /* ../../rtl/core/neorv32_top.vhd:513:23  */
  assign n272 = firq[10]; // extract
  /* ../../rtl/core/neorv32_top.vhd:514:23  */
  assign n273 = firq[5]; // extract
  /* ../../rtl/core/neorv32_top.vhd:515:23  */
  assign n274 = firq[4]; // extract
  /* ../../rtl/core/neorv32_top.vhd:516:23  */
  assign n275 = firq[2]; // extract
  /* ../../rtl/core/neorv32_top.vhd:517:23  */
  assign n276 = firq[1]; // extract
  /* ../../rtl/core/neorv32_top.vhd:525:5  */
  neorv32_cpu_0_0_0_4_0_64_0_6c34b6f631e21924a9ad40faece5c46447e85d2d core_complex_gen_n1_neorv32_cpu_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_sys),
    .mtime_i(mtime),
    .msi_i(msi),
    .mei_i(irq_mei_i),
    .mti_i(mti),
    .firq_i(cpu_firq),
    .dbi_i(dci_haltreq),
    .\ibus_rsp_i_ibus_rsp_i[ack] (n282),
    .\ibus_rsp_i_ibus_rsp_i[err] (n283),
    .\ibus_rsp_i_ibus_rsp_i[data] (n284),
    .\dbus_rsp_i_dbus_rsp_i[ack] (n287),
    .\dbus_rsp_i_dbus_rsp_i[err] (n288),
    .\dbus_rsp_i_dbus_rsp_i[data] (n289),
    .\trace_o_trace_o[valid] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[valid] ),
    .\trace_o_trace_o[order] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[order] ),
    .\trace_o_trace_o[insn] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[insn] ),
    .\trace_o_trace_o[trap] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[trap] ),
    .\trace_o_trace_o[halt] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[halt] ),
    .\trace_o_trace_o[intr] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[intr] ),
    .\trace_o_trace_o[mode] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mode] ),
    .\trace_o_trace_o[ixl] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[ixl] ),
    .\trace_o_trace_o[debug] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[debug] ),
    .\trace_o_trace_o[compr] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[compr] ),
    .\trace_o_trace_o[delta] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[delta] ),
    .\trace_o_trace_o[cmd32] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[cmd32] ),
    .\trace_o_trace_o[rs1_addr] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs1_addr] ),
    .\trace_o_trace_o[rs2_addr] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs2_addr] ),
    .\trace_o_trace_o[rs1_rdata] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs1_rdata] ),
    .\trace_o_trace_o[rs2_rdata] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs2_rdata] ),
    .\trace_o_trace_o[rd_addr] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rd_addr] ),
    .\trace_o_trace_o[rd_rdata] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rd_rdata] ),
    .\trace_o_trace_o[pc_rdata] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[pc_rdata] ),
    .\trace_o_trace_o[pc_wdata] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[pc_wdata] ),
    .\trace_o_trace_o[csr_addr] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[csr_addr] ),
    .\trace_o_trace_o[csr_rdata] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[csr_rdata] ),
    .\trace_o_trace_o[csr_wdata] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[csr_wdata] ),
    .\trace_o_trace_o[mem_addr] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_addr] ),
    .\trace_o_trace_o[mem_rmask] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_rmask] ),
    .\trace_o_trace_o[mem_wmask] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_wmask] ),
    .\trace_o_trace_o[mem_rdata] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_rdata] ),
    .\trace_o_trace_o[mem_wdata] (\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_wdata] ),
    .sleep_o(),
    .fence_o(),
    .\ibus_req_o_ibus_req_o[meta] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[meta] ),
    .\ibus_req_o_ibus_req_o[addr] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[addr] ),
    .\ibus_req_o_ibus_req_o[data] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[data] ),
    .\ibus_req_o_ibus_req_o[ben] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[ben] ),
    .\ibus_req_o_ibus_req_o[stb] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[stb] ),
    .\ibus_req_o_ibus_req_o[rw] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[rw] ),
    .\ibus_req_o_ibus_req_o[amo] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[amo] ),
    .\ibus_req_o_ibus_req_o[amoop] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[amoop] ),
    .\ibus_req_o_ibus_req_o[burst] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[burst] ),
    .\ibus_req_o_ibus_req_o[lock] (\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[lock] ),
    .\dbus_req_o_dbus_req_o[meta] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[meta] ),
    .\dbus_req_o_dbus_req_o[addr] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[addr] ),
    .\dbus_req_o_dbus_req_o[data] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[data] ),
    .\dbus_req_o_dbus_req_o[ben] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[ben] ),
    .\dbus_req_o_dbus_req_o[stb] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[stb] ),
    .\dbus_req_o_dbus_req_o[rw] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[rw] ),
    .\dbus_req_o_dbus_req_o[amo] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[amo] ),
    .\dbus_req_o_dbus_req_o[amoop] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[amoop] ),
    .\dbus_req_o_dbus_req_o[burst] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[burst] ),
    .\dbus_req_o_dbus_req_o[lock] (\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[lock] ));
  assign n277 = {\core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_wdata] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_rdata] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_wmask] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_rmask] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mem_addr] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[csr_wdata] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[csr_rdata] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[csr_addr] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[pc_wdata] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[pc_rdata] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rd_rdata] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rd_addr] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs2_rdata] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs1_rdata] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs2_addr] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[rs1_addr] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[cmd32] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[delta] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[compr] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[debug] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[ixl] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[mode] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[intr] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[halt] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[trap] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[insn] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[order] , \core_complex_gen_n1_neorv32_cpu_inst.trace_o_trace_o[valid] };
  assign n280 = {\core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[lock] , \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[burst] , \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[amoop] , \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[amo] , \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[rw] , \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[stb] , \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[ben] , \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[data] , \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[addr] , \core_complex_gen_n1_neorv32_cpu_inst.ibus_req_o_ibus_req_o[meta] };
  assign n282 = cpu_i_rsp[0]; // extract
  assign n283 = cpu_i_rsp[1]; // extract
  assign n284 = cpu_i_rsp[33:2]; // extract
  assign n285 = {\core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[lock] , \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[burst] , \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[amoop] , \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[amo] , \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[rw] , \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[stb] , \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[ben] , \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[data] , \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[addr] , \core_complex_gen_n1_neorv32_cpu_inst.dbus_req_o_dbus_req_o[meta] };
  assign n287 = cpu_d_rsp[0]; // extract
  assign n288 = cpu_d_rsp[1]; // extract
  assign n289 = cpu_d_rsp[33:2]; // extract
  /* ../../rtl/core/neorv32_top.vhd:665:5  */
  neorv32_bus_switch_2547cc736e951fa4919853c43ae890861a3b3264 core_complex_gen_n1_neorv32_core_bus_switch_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_sys),
    .\a_req_i_a_req_i[meta] (n290),
    .\a_req_i_a_req_i[addr] (n291),
    .\a_req_i_a_req_i[data] (n292),
    .\a_req_i_a_req_i[ben] (n293),
    .\a_req_i_a_req_i[stb] (n294),
    .\a_req_i_a_req_i[rw] (n295),
    .\a_req_i_a_req_i[amo] (n296),
    .\a_req_i_a_req_i[amoop] (n297),
    .\a_req_i_a_req_i[burst] (n298),
    .\a_req_i_a_req_i[lock] (n299),
    .\b_req_i_b_req_i[meta] (n302),
    .\b_req_i_b_req_i[addr] (n303),
    .\b_req_i_b_req_i[data] (n304),
    .\b_req_i_b_req_i[ben] (n305),
    .\b_req_i_b_req_i[stb] (n306),
    .\b_req_i_b_req_i[rw] (n307),
    .\b_req_i_b_req_i[amo] (n308),
    .\b_req_i_b_req_i[amoop] (n309),
    .\b_req_i_b_req_i[burst] (n310),
    .\b_req_i_b_req_i[lock] (n311),
    .\x_rsp_i_x_rsp_i[ack] (n316),
    .\x_rsp_i_x_rsp_i[err] (n317),
    .\x_rsp_i_x_rsp_i[data] (n318),
    .\a_rsp_o_a_rsp_o[ack] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.a_rsp_o_a_rsp_o[ack] ),
    .\a_rsp_o_a_rsp_o[err] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.a_rsp_o_a_rsp_o[err] ),
    .\a_rsp_o_a_rsp_o[data] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.a_rsp_o_a_rsp_o[data] ),
    .\b_rsp_o_b_rsp_o[ack] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.b_rsp_o_b_rsp_o[ack] ),
    .\b_rsp_o_b_rsp_o[err] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.b_rsp_o_b_rsp_o[err] ),
    .\b_rsp_o_b_rsp_o[data] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.b_rsp_o_b_rsp_o[data] ),
    .\x_req_o_x_req_o[meta] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[meta] ),
    .\x_req_o_x_req_o[addr] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[addr] ),
    .\x_req_o_x_req_o[data] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[data] ),
    .\x_req_o_x_req_o[ben] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[ben] ),
    .\x_req_o_x_req_o[stb] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[stb] ),
    .\x_req_o_x_req_o[rw] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[rw] ),
    .\x_req_o_x_req_o[amo] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[amo] ),
    .\x_req_o_x_req_o[amoop] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[amoop] ),
    .\x_req_o_x_req_o[burst] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[burst] ),
    .\x_req_o_x_req_o[lock] (\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[lock] ));
  assign n290 = dcache_req[4:0]; // extract
  assign n291 = dcache_req[36:5]; // extract
  assign n292 = dcache_req[68:37]; // extract
  assign n293 = dcache_req[72:69]; // extract
  assign n294 = dcache_req[73]; // extract
  assign n295 = dcache_req[74]; // extract
  assign n296 = dcache_req[75]; // extract
  assign n297 = dcache_req[79:76]; // extract
  assign n298 = dcache_req[80]; // extract
  assign n299 = dcache_req[81]; // extract
  assign n300 = {\core_complex_gen_n1_neorv32_core_bus_switch_inst.a_rsp_o_a_rsp_o[data] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.a_rsp_o_a_rsp_o[err] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.a_rsp_o_a_rsp_o[ack] };
  assign n302 = icache_req[4:0]; // extract
  assign n303 = icache_req[36:5]; // extract
  assign n304 = icache_req[68:37]; // extract
  assign n305 = icache_req[72:69]; // extract
  assign n306 = icache_req[73]; // extract
  assign n307 = icache_req[74]; // extract
  assign n308 = icache_req[75]; // extract
  assign n309 = icache_req[79:76]; // extract
  assign n310 = icache_req[80]; // extract
  assign n311 = icache_req[81]; // extract
  assign n312 = {\core_complex_gen_n1_neorv32_core_bus_switch_inst.b_rsp_o_b_rsp_o[data] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.b_rsp_o_b_rsp_o[err] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.b_rsp_o_b_rsp_o[ack] };
  assign n314 = {\core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[lock] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[burst] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[amoop] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[amo] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[rw] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[stb] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[ben] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[data] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[addr] , \core_complex_gen_n1_neorv32_core_bus_switch_inst.x_req_o_x_req_o[meta] };
  assign n316 = core_rsp[0]; // extract
  assign n317 = core_rsp[1]; // extract
  assign n318 = core_rsp[33:2]; // extract
  /* ../../rtl/core/neorv32_top.vhd:686:45  */
  assign n320 = 1'b0 ? cpu_trace : 462'b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000;
  /* ../../rtl/core/neorv32_top.vhd:825:3  */
  neorv32_bus_gateway_16_0_16384_8192_2097152_31db79d4b6fe03934eb4ff891fa609cb3097642e neorv32_bus_gateway_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_sys),
    .\req_i_req_i[meta] (n327),
    .\req_i_req_i[addr] (n328),
    .\req_i_req_i[data] (n329),
    .\req_i_req_i[ben] (n330),
    .\req_i_req_i[stb] (n331),
    .\req_i_req_i[rw] (n332),
    .\req_i_req_i[amo] (n333),
    .\req_i_req_i[amoop] (n334),
    .\req_i_req_i[burst] (n335),
    .\req_i_req_i[lock] (n336),
    .\a_rsp_i_a_rsp_i[ack] (n341),
    .\a_rsp_i_a_rsp_i[err] (n342),
    .\a_rsp_i_a_rsp_i[data] (n343),
    .\b_rsp_i_b_rsp_i[ack] (n346),
    .\b_rsp_i_b_rsp_i[err] (n347),
    .\b_rsp_i_b_rsp_i[data] (n348),
    .\c_rsp_i_c_rsp_i[ack] (n351),
    .\c_rsp_i_c_rsp_i[err] (n352),
    .\c_rsp_i_c_rsp_i[data] (n353),
    .\x_rsp_i_x_rsp_i[ack] (n356),
    .\x_rsp_i_x_rsp_i[err] (n357),
    .\x_rsp_i_x_rsp_i[data] (n358),
    .term_o(xbus_terminate),
    .\rsp_o_rsp_o[ack] (\neorv32_bus_gateway_inst.rsp_o_rsp_o[ack] ),
    .\rsp_o_rsp_o[err] (\neorv32_bus_gateway_inst.rsp_o_rsp_o[err] ),
    .\rsp_o_rsp_o[data] (\neorv32_bus_gateway_inst.rsp_o_rsp_o[data] ),
    .\a_req_o_a_req_o[meta] (),
    .\a_req_o_a_req_o[addr] (),
    .\a_req_o_a_req_o[data] (),
    .\a_req_o_a_req_o[ben] (),
    .\a_req_o_a_req_o[stb] (),
    .\a_req_o_a_req_o[rw] (),
    .\a_req_o_a_req_o[amo] (),
    .\a_req_o_a_req_o[amoop] (),
    .\a_req_o_a_req_o[burst] (),
    .\a_req_o_a_req_o[lock] (),
    .\b_req_o_b_req_o[meta] (),
    .\b_req_o_b_req_o[addr] (),
    .\b_req_o_b_req_o[data] (),
    .\b_req_o_b_req_o[ben] (),
    .\b_req_o_b_req_o[stb] (),
    .\b_req_o_b_req_o[rw] (),
    .\b_req_o_b_req_o[amo] (),
    .\b_req_o_b_req_o[amoop] (),
    .\b_req_o_b_req_o[burst] (),
    .\b_req_o_b_req_o[lock] (),
    .\c_req_o_c_req_o[meta] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[meta] ),
    .\c_req_o_c_req_o[addr] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[addr] ),
    .\c_req_o_c_req_o[data] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[data] ),
    .\c_req_o_c_req_o[ben] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[ben] ),
    .\c_req_o_c_req_o[stb] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[stb] ),
    .\c_req_o_c_req_o[rw] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[rw] ),
    .\c_req_o_c_req_o[amo] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[amo] ),
    .\c_req_o_c_req_o[amoop] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[amoop] ),
    .\c_req_o_c_req_o[burst] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[burst] ),
    .\c_req_o_c_req_o[lock] (\neorv32_bus_gateway_inst.c_req_o_c_req_o[lock] ),
    .\x_req_o_x_req_o[meta] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[meta] ),
    .\x_req_o_x_req_o[addr] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[addr] ),
    .\x_req_o_x_req_o[data] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[data] ),
    .\x_req_o_x_req_o[ben] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[ben] ),
    .\x_req_o_x_req_o[stb] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[stb] ),
    .\x_req_o_x_req_o[rw] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[rw] ),
    .\x_req_o_x_req_o[amo] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[amo] ),
    .\x_req_o_x_req_o[amoop] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[amoop] ),
    .\x_req_o_x_req_o[burst] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[burst] ),
    .\x_req_o_x_req_o[lock] (\neorv32_bus_gateway_inst.x_req_o_x_req_o[lock] ));
  assign n327 = sys3_req[4:0]; // extract
  assign n328 = sys3_req[36:5]; // extract
  assign n329 = sys3_req[68:37]; // extract
  assign n330 = sys3_req[72:69]; // extract
  assign n331 = sys3_req[73]; // extract
  assign n332 = sys3_req[74]; // extract
  assign n333 = sys3_req[75]; // extract
  assign n334 = sys3_req[79:76]; // extract
  assign n335 = sys3_req[80]; // extract
  assign n336 = sys3_req[81]; // extract
  assign n337 = {\neorv32_bus_gateway_inst.rsp_o_rsp_o[data] , \neorv32_bus_gateway_inst.rsp_o_rsp_o[err] , \neorv32_bus_gateway_inst.rsp_o_rsp_o[ack] };
  assign n341 = imem_rsp[0]; // extract
  assign n342 = imem_rsp[1]; // extract
  assign n343 = imem_rsp[33:2]; // extract
  assign n346 = dmem_rsp[0]; // extract
  assign n347 = dmem_rsp[1]; // extract
  assign n348 = dmem_rsp[33:2]; // extract
  assign n349 = {\neorv32_bus_gateway_inst.c_req_o_c_req_o[lock] , \neorv32_bus_gateway_inst.c_req_o_c_req_o[burst] , \neorv32_bus_gateway_inst.c_req_o_c_req_o[amoop] , \neorv32_bus_gateway_inst.c_req_o_c_req_o[amo] , \neorv32_bus_gateway_inst.c_req_o_c_req_o[rw] , \neorv32_bus_gateway_inst.c_req_o_c_req_o[stb] , \neorv32_bus_gateway_inst.c_req_o_c_req_o[ben] , \neorv32_bus_gateway_inst.c_req_o_c_req_o[data] , \neorv32_bus_gateway_inst.c_req_o_c_req_o[addr] , \neorv32_bus_gateway_inst.c_req_o_c_req_o[meta] };
  assign n351 = io_rsp[0]; // extract
  assign n352 = io_rsp[1]; // extract
  assign n353 = io_rsp[33:2]; // extract
  assign n354 = {\neorv32_bus_gateway_inst.x_req_o_x_req_o[lock] , \neorv32_bus_gateway_inst.x_req_o_x_req_o[burst] , \neorv32_bus_gateway_inst.x_req_o_x_req_o[amoop] , \neorv32_bus_gateway_inst.x_req_o_x_req_o[amo] , \neorv32_bus_gateway_inst.x_req_o_x_req_o[rw] , \neorv32_bus_gateway_inst.x_req_o_x_req_o[stb] , \neorv32_bus_gateway_inst.x_req_o_x_req_o[ben] , \neorv32_bus_gateway_inst.x_req_o_x_req_o[data] , \neorv32_bus_gateway_inst.x_req_o_x_req_o[addr] , \neorv32_bus_gateway_inst.x_req_o_x_req_o[meta] };
  assign n356 = xbus_rsp[0]; // extract
  assign n357 = xbus_rsp[1]; // extract
  assign n358 = xbus_rsp[33:2]; // extract
  /* ../../rtl/core/neorv32_top.vhd:919:7  */
  neorv32_xbus_5ba93c9db0cff93f52b521d7420e43f6eda2784f memory_system_neorv32_xbus_enabled_neorv32_xbus_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_sys),
    .bus_term_i(xbus_terminate),
    .\bus_req_i_bus_req_i[meta] (n361),
    .\bus_req_i_bus_req_i[addr] (n362),
    .\bus_req_i_bus_req_i[data] (n363),
    .\bus_req_i_bus_req_i[ben] (n364),
    .\bus_req_i_bus_req_i[stb] (n365),
    .\bus_req_i_bus_req_i[rw] (n366),
    .\bus_req_i_bus_req_i[amo] (n367),
    .\bus_req_i_bus_req_i[amoop] (n368),
    .\bus_req_i_bus_req_i[burst] (n369),
    .\bus_req_i_bus_req_i[lock] (n370),
    .xbus_dat_i(xbus_dat_i),
    .xbus_ack_i(xbus_ack_i),
    .xbus_err_i(xbus_err_i),
    .\bus_rsp_o_bus_rsp_o[ack] (\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.bus_rsp_o_bus_rsp_o[ack] ),
    .\bus_rsp_o_bus_rsp_o[err] (\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.bus_rsp_o_bus_rsp_o[err] ),
    .\bus_rsp_o_bus_rsp_o[data] (\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.bus_rsp_o_bus_rsp_o[data] ),
    .xbus_adr_o(\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_adr_o ),
    .xbus_dat_o(\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_dat_o ),
    .xbus_cti_o(\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_cti_o ),
    .xbus_tag_o(\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_tag_o ),
    .xbus_we_o(\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_we_o ),
    .xbus_sel_o(\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_sel_o ),
    .xbus_stb_o(\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_stb_o ),
    .xbus_cyc_o(\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.xbus_cyc_o ));
  assign n361 = xbus_req[4:0]; // extract
  assign n362 = xbus_req[36:5]; // extract
  assign n363 = xbus_req[68:37]; // extract
  assign n364 = xbus_req[72:69]; // extract
  assign n365 = xbus_req[73]; // extract
  assign n366 = xbus_req[74]; // extract
  assign n367 = xbus_req[75]; // extract
  assign n368 = xbus_req[79:76]; // extract
  assign n369 = xbus_req[80]; // extract
  assign n370 = xbus_req[81]; // extract
  assign n371 = {\memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.bus_rsp_o_bus_rsp_o[data] , \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.bus_rsp_o_bus_rsp_o[err] , \memory_system_neorv32_xbus_enabled_neorv32_xbus_inst.bus_rsp_o_bus_rsp_o[ack] };
  /* ../../rtl/core/neorv32_top.vhd:967:5  */
  neorv32_bus_io_switch_65536_2c202e980184e67cda5d0f34d3be5ef651a6fcca io_system_neorv32_bus_io_switch_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_sys),
    .\main_req_i_main_req_i[meta] (n381),
    .\main_req_i_main_req_i[addr] (n382),
    .\main_req_i_main_req_i[data] (n383),
    .\main_req_i_main_req_i[ben] (n384),
    .\main_req_i_main_req_i[stb] (n385),
    .\main_req_i_main_req_i[rw] (n386),
    .\main_req_i_main_req_i[amo] (n387),
    .\main_req_i_main_req_i[amoop] (n388),
    .\main_req_i_main_req_i[burst] (n389),
    .\main_req_i_main_req_i[lock] (n390),
    .\dev_00_rsp_i_dev_00_rsp_i[ack] (n396),
    .\dev_00_rsp_i_dev_00_rsp_i[err] (n397),
    .\dev_00_rsp_i_dev_00_rsp_i[data] (n398),
    .\dev_01_rsp_i_dev_01_rsp_i[ack] (n400),
    .\dev_01_rsp_i_dev_01_rsp_i[err] (n401),
    .\dev_01_rsp_i_dev_01_rsp_i[data] (n402),
    .\dev_02_rsp_i_dev_02_rsp_i[ack] (n404),
    .\dev_02_rsp_i_dev_02_rsp_i[err] (n405),
    .\dev_02_rsp_i_dev_02_rsp_i[data] (n406),
    .\dev_03_rsp_i_dev_03_rsp_i[ack] (n408),
    .\dev_03_rsp_i_dev_03_rsp_i[err] (n409),
    .\dev_03_rsp_i_dev_03_rsp_i[data] (n410),
    .\dev_04_rsp_i_dev_04_rsp_i[ack] (n412),
    .\dev_04_rsp_i_dev_04_rsp_i[err] (n413),
    .\dev_04_rsp_i_dev_04_rsp_i[data] (n414),
    .\dev_05_rsp_i_dev_05_rsp_i[ack] (n416),
    .\dev_05_rsp_i_dev_05_rsp_i[err] (n417),
    .\dev_05_rsp_i_dev_05_rsp_i[data] (n418),
    .\dev_06_rsp_i_dev_06_rsp_i[ack] (n420),
    .\dev_06_rsp_i_dev_06_rsp_i[err] (n421),
    .\dev_06_rsp_i_dev_06_rsp_i[data] (n422),
    .\dev_07_rsp_i_dev_07_rsp_i[ack] (n424),
    .\dev_07_rsp_i_dev_07_rsp_i[err] (n425),
    .\dev_07_rsp_i_dev_07_rsp_i[data] (n426),
    .\dev_08_rsp_i_dev_08_rsp_i[ack] (n428),
    .\dev_08_rsp_i_dev_08_rsp_i[err] (n429),
    .\dev_08_rsp_i_dev_08_rsp_i[data] (n430),
    .\dev_09_rsp_i_dev_09_rsp_i[ack] (n432),
    .\dev_09_rsp_i_dev_09_rsp_i[err] (n433),
    .\dev_09_rsp_i_dev_09_rsp_i[data] (n434),
    .\dev_10_rsp_i_dev_10_rsp_i[ack] (n438),
    .\dev_10_rsp_i_dev_10_rsp_i[err] (n439),
    .\dev_10_rsp_i_dev_10_rsp_i[data] (n440),
    .\dev_11_rsp_i_dev_11_rsp_i[ack] (n444),
    .\dev_11_rsp_i_dev_11_rsp_i[err] (n445),
    .\dev_11_rsp_i_dev_11_rsp_i[data] (n446),
    .\dev_12_rsp_i_dev_12_rsp_i[ack] (n450),
    .\dev_12_rsp_i_dev_12_rsp_i[err] (n451),
    .\dev_12_rsp_i_dev_12_rsp_i[data] (n452),
    .\dev_13_rsp_i_dev_13_rsp_i[ack] (n456),
    .\dev_13_rsp_i_dev_13_rsp_i[err] (n457),
    .\dev_13_rsp_i_dev_13_rsp_i[data] (n458),
    .\dev_14_rsp_i_dev_14_rsp_i[ack] (n460),
    .\dev_14_rsp_i_dev_14_rsp_i[err] (n461),
    .\dev_14_rsp_i_dev_14_rsp_i[data] (n462),
    .\dev_15_rsp_i_dev_15_rsp_i[ack] (n464),
    .\dev_15_rsp_i_dev_15_rsp_i[err] (n465),
    .\dev_15_rsp_i_dev_15_rsp_i[data] (n466),
    .\dev_16_rsp_i_dev_16_rsp_i[ack] (n470),
    .\dev_16_rsp_i_dev_16_rsp_i[err] (n471),
    .\dev_16_rsp_i_dev_16_rsp_i[data] (n472),
    .\dev_17_rsp_i_dev_17_rsp_i[ack] (n476),
    .\dev_17_rsp_i_dev_17_rsp_i[err] (n477),
    .\dev_17_rsp_i_dev_17_rsp_i[data] (n478),
    .\dev_18_rsp_i_dev_18_rsp_i[ack] (n482),
    .\dev_18_rsp_i_dev_18_rsp_i[err] (n483),
    .\dev_18_rsp_i_dev_18_rsp_i[data] (n484),
    .\dev_19_rsp_i_dev_19_rsp_i[ack] (n488),
    .\dev_19_rsp_i_dev_19_rsp_i[err] (n489),
    .\dev_19_rsp_i_dev_19_rsp_i[data] (n490),
    .\dev_20_rsp_i_dev_20_rsp_i[ack] (n494),
    .\dev_20_rsp_i_dev_20_rsp_i[err] (n495),
    .\dev_20_rsp_i_dev_20_rsp_i[data] (n496),
    .\dev_21_rsp_i_dev_21_rsp_i[ack] (n500),
    .\dev_21_rsp_i_dev_21_rsp_i[err] (n501),
    .\dev_21_rsp_i_dev_21_rsp_i[data] (n502),
    .\dev_22_rsp_i_dev_22_rsp_i[ack] (n506),
    .\dev_22_rsp_i_dev_22_rsp_i[err] (n507),
    .\dev_22_rsp_i_dev_22_rsp_i[data] (n508),
    .\dev_23_rsp_i_dev_23_rsp_i[ack] (n512),
    .\dev_23_rsp_i_dev_23_rsp_i[err] (n513),
    .\dev_23_rsp_i_dev_23_rsp_i[data] (n514),
    .\dev_24_rsp_i_dev_24_rsp_i[ack] (n518),
    .\dev_24_rsp_i_dev_24_rsp_i[err] (n519),
    .\dev_24_rsp_i_dev_24_rsp_i[data] (n520),
    .\dev_25_rsp_i_dev_25_rsp_i[ack] (n524),
    .\dev_25_rsp_i_dev_25_rsp_i[err] (n525),
    .\dev_25_rsp_i_dev_25_rsp_i[data] (n526),
    .\dev_26_rsp_i_dev_26_rsp_i[ack] (n530),
    .\dev_26_rsp_i_dev_26_rsp_i[err] (n531),
    .\dev_26_rsp_i_dev_26_rsp_i[data] (n532),
    .\dev_27_rsp_i_dev_27_rsp_i[ack] (n536),
    .\dev_27_rsp_i_dev_27_rsp_i[err] (n537),
    .\dev_27_rsp_i_dev_27_rsp_i[data] (n538),
    .\dev_28_rsp_i_dev_28_rsp_i[ack] (n542),
    .\dev_28_rsp_i_dev_28_rsp_i[err] (n543),
    .\dev_28_rsp_i_dev_28_rsp_i[data] (n544),
    .\dev_29_rsp_i_dev_29_rsp_i[ack] (n548),
    .\dev_29_rsp_i_dev_29_rsp_i[err] (n549),
    .\dev_29_rsp_i_dev_29_rsp_i[data] (n550),
    .\dev_30_rsp_i_dev_30_rsp_i[ack] (n554),
    .\dev_30_rsp_i_dev_30_rsp_i[err] (n555),
    .\dev_30_rsp_i_dev_30_rsp_i[data] (n556),
    .\dev_31_rsp_i_dev_31_rsp_i[ack] (n560),
    .\dev_31_rsp_i_dev_31_rsp_i[err] (n561),
    .\dev_31_rsp_i_dev_31_rsp_i[data] (n562),
    .\main_rsp_o_main_rsp_o[ack] (\io_system_neorv32_bus_io_switch_inst.main_rsp_o_main_rsp_o[ack] ),
    .\main_rsp_o_main_rsp_o[err] (\io_system_neorv32_bus_io_switch_inst.main_rsp_o_main_rsp_o[err] ),
    .\main_rsp_o_main_rsp_o[data] (\io_system_neorv32_bus_io_switch_inst.main_rsp_o_main_rsp_o[data] ),
    .\dev_00_req_o_dev_00_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[meta] ),
    .\dev_00_req_o_dev_00_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[addr] ),
    .\dev_00_req_o_dev_00_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[data] ),
    .\dev_00_req_o_dev_00_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[ben] ),
    .\dev_00_req_o_dev_00_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[stb] ),
    .\dev_00_req_o_dev_00_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[rw] ),
    .\dev_00_req_o_dev_00_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[amo] ),
    .\dev_00_req_o_dev_00_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[amoop] ),
    .\dev_00_req_o_dev_00_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[burst] ),
    .\dev_00_req_o_dev_00_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[lock] ),
    .\dev_01_req_o_dev_01_req_o[meta] (),
    .\dev_01_req_o_dev_01_req_o[addr] (),
    .\dev_01_req_o_dev_01_req_o[data] (),
    .\dev_01_req_o_dev_01_req_o[ben] (),
    .\dev_01_req_o_dev_01_req_o[stb] (),
    .\dev_01_req_o_dev_01_req_o[rw] (),
    .\dev_01_req_o_dev_01_req_o[amo] (),
    .\dev_01_req_o_dev_01_req_o[amoop] (),
    .\dev_01_req_o_dev_01_req_o[burst] (),
    .\dev_01_req_o_dev_01_req_o[lock] (),
    .\dev_02_req_o_dev_02_req_o[meta] (),
    .\dev_02_req_o_dev_02_req_o[addr] (),
    .\dev_02_req_o_dev_02_req_o[data] (),
    .\dev_02_req_o_dev_02_req_o[ben] (),
    .\dev_02_req_o_dev_02_req_o[stb] (),
    .\dev_02_req_o_dev_02_req_o[rw] (),
    .\dev_02_req_o_dev_02_req_o[amo] (),
    .\dev_02_req_o_dev_02_req_o[amoop] (),
    .\dev_02_req_o_dev_02_req_o[burst] (),
    .\dev_02_req_o_dev_02_req_o[lock] (),
    .\dev_03_req_o_dev_03_req_o[meta] (),
    .\dev_03_req_o_dev_03_req_o[addr] (),
    .\dev_03_req_o_dev_03_req_o[data] (),
    .\dev_03_req_o_dev_03_req_o[ben] (),
    .\dev_03_req_o_dev_03_req_o[stb] (),
    .\dev_03_req_o_dev_03_req_o[rw] (),
    .\dev_03_req_o_dev_03_req_o[amo] (),
    .\dev_03_req_o_dev_03_req_o[amoop] (),
    .\dev_03_req_o_dev_03_req_o[burst] (),
    .\dev_03_req_o_dev_03_req_o[lock] (),
    .\dev_04_req_o_dev_04_req_o[meta] (),
    .\dev_04_req_o_dev_04_req_o[addr] (),
    .\dev_04_req_o_dev_04_req_o[data] (),
    .\dev_04_req_o_dev_04_req_o[ben] (),
    .\dev_04_req_o_dev_04_req_o[stb] (),
    .\dev_04_req_o_dev_04_req_o[rw] (),
    .\dev_04_req_o_dev_04_req_o[amo] (),
    .\dev_04_req_o_dev_04_req_o[amoop] (),
    .\dev_04_req_o_dev_04_req_o[burst] (),
    .\dev_04_req_o_dev_04_req_o[lock] (),
    .\dev_05_req_o_dev_05_req_o[meta] (),
    .\dev_05_req_o_dev_05_req_o[addr] (),
    .\dev_05_req_o_dev_05_req_o[data] (),
    .\dev_05_req_o_dev_05_req_o[ben] (),
    .\dev_05_req_o_dev_05_req_o[stb] (),
    .\dev_05_req_o_dev_05_req_o[rw] (),
    .\dev_05_req_o_dev_05_req_o[amo] (),
    .\dev_05_req_o_dev_05_req_o[amoop] (),
    .\dev_05_req_o_dev_05_req_o[burst] (),
    .\dev_05_req_o_dev_05_req_o[lock] (),
    .\dev_06_req_o_dev_06_req_o[meta] (),
    .\dev_06_req_o_dev_06_req_o[addr] (),
    .\dev_06_req_o_dev_06_req_o[data] (),
    .\dev_06_req_o_dev_06_req_o[ben] (),
    .\dev_06_req_o_dev_06_req_o[stb] (),
    .\dev_06_req_o_dev_06_req_o[rw] (),
    .\dev_06_req_o_dev_06_req_o[amo] (),
    .\dev_06_req_o_dev_06_req_o[amoop] (),
    .\dev_06_req_o_dev_06_req_o[burst] (),
    .\dev_06_req_o_dev_06_req_o[lock] (),
    .\dev_07_req_o_dev_07_req_o[meta] (),
    .\dev_07_req_o_dev_07_req_o[addr] (),
    .\dev_07_req_o_dev_07_req_o[data] (),
    .\dev_07_req_o_dev_07_req_o[ben] (),
    .\dev_07_req_o_dev_07_req_o[stb] (),
    .\dev_07_req_o_dev_07_req_o[rw] (),
    .\dev_07_req_o_dev_07_req_o[amo] (),
    .\dev_07_req_o_dev_07_req_o[amoop] (),
    .\dev_07_req_o_dev_07_req_o[burst] (),
    .\dev_07_req_o_dev_07_req_o[lock] (),
    .\dev_08_req_o_dev_08_req_o[meta] (),
    .\dev_08_req_o_dev_08_req_o[addr] (),
    .\dev_08_req_o_dev_08_req_o[data] (),
    .\dev_08_req_o_dev_08_req_o[ben] (),
    .\dev_08_req_o_dev_08_req_o[stb] (),
    .\dev_08_req_o_dev_08_req_o[rw] (),
    .\dev_08_req_o_dev_08_req_o[amo] (),
    .\dev_08_req_o_dev_08_req_o[amoop] (),
    .\dev_08_req_o_dev_08_req_o[burst] (),
    .\dev_08_req_o_dev_08_req_o[lock] (),
    .\dev_09_req_o_dev_09_req_o[meta] (),
    .\dev_09_req_o_dev_09_req_o[addr] (),
    .\dev_09_req_o_dev_09_req_o[data] (),
    .\dev_09_req_o_dev_09_req_o[ben] (),
    .\dev_09_req_o_dev_09_req_o[stb] (),
    .\dev_09_req_o_dev_09_req_o[rw] (),
    .\dev_09_req_o_dev_09_req_o[amo] (),
    .\dev_09_req_o_dev_09_req_o[amoop] (),
    .\dev_09_req_o_dev_09_req_o[burst] (),
    .\dev_09_req_o_dev_09_req_o[lock] (),
    .\dev_10_req_o_dev_10_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[meta] ),
    .\dev_10_req_o_dev_10_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[addr] ),
    .\dev_10_req_o_dev_10_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[data] ),
    .\dev_10_req_o_dev_10_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[ben] ),
    .\dev_10_req_o_dev_10_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[stb] ),
    .\dev_10_req_o_dev_10_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[rw] ),
    .\dev_10_req_o_dev_10_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[amo] ),
    .\dev_10_req_o_dev_10_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[amoop] ),
    .\dev_10_req_o_dev_10_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[burst] ),
    .\dev_10_req_o_dev_10_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[lock] ),
    .\dev_11_req_o_dev_11_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[meta] ),
    .\dev_11_req_o_dev_11_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[addr] ),
    .\dev_11_req_o_dev_11_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[data] ),
    .\dev_11_req_o_dev_11_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[ben] ),
    .\dev_11_req_o_dev_11_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[stb] ),
    .\dev_11_req_o_dev_11_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[rw] ),
    .\dev_11_req_o_dev_11_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[amo] ),
    .\dev_11_req_o_dev_11_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[amoop] ),
    .\dev_11_req_o_dev_11_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[burst] ),
    .\dev_11_req_o_dev_11_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[lock] ),
    .\dev_12_req_o_dev_12_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[meta] ),
    .\dev_12_req_o_dev_12_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[addr] ),
    .\dev_12_req_o_dev_12_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[data] ),
    .\dev_12_req_o_dev_12_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[ben] ),
    .\dev_12_req_o_dev_12_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[stb] ),
    .\dev_12_req_o_dev_12_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[rw] ),
    .\dev_12_req_o_dev_12_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[amo] ),
    .\dev_12_req_o_dev_12_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[amoop] ),
    .\dev_12_req_o_dev_12_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[burst] ),
    .\dev_12_req_o_dev_12_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[lock] ),
    .\dev_13_req_o_dev_13_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[meta] ),
    .\dev_13_req_o_dev_13_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[addr] ),
    .\dev_13_req_o_dev_13_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[data] ),
    .\dev_13_req_o_dev_13_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[ben] ),
    .\dev_13_req_o_dev_13_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[stb] ),
    .\dev_13_req_o_dev_13_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[rw] ),
    .\dev_13_req_o_dev_13_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[amo] ),
    .\dev_13_req_o_dev_13_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[amoop] ),
    .\dev_13_req_o_dev_13_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[burst] ),
    .\dev_13_req_o_dev_13_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[lock] ),
    .\dev_14_req_o_dev_14_req_o[meta] (),
    .\dev_14_req_o_dev_14_req_o[addr] (),
    .\dev_14_req_o_dev_14_req_o[data] (),
    .\dev_14_req_o_dev_14_req_o[ben] (),
    .\dev_14_req_o_dev_14_req_o[stb] (),
    .\dev_14_req_o_dev_14_req_o[rw] (),
    .\dev_14_req_o_dev_14_req_o[amo] (),
    .\dev_14_req_o_dev_14_req_o[amoop] (),
    .\dev_14_req_o_dev_14_req_o[burst] (),
    .\dev_14_req_o_dev_14_req_o[lock] (),
    .\dev_15_req_o_dev_15_req_o[meta] (),
    .\dev_15_req_o_dev_15_req_o[addr] (),
    .\dev_15_req_o_dev_15_req_o[data] (),
    .\dev_15_req_o_dev_15_req_o[ben] (),
    .\dev_15_req_o_dev_15_req_o[stb] (),
    .\dev_15_req_o_dev_15_req_o[rw] (),
    .\dev_15_req_o_dev_15_req_o[amo] (),
    .\dev_15_req_o_dev_15_req_o[amoop] (),
    .\dev_15_req_o_dev_15_req_o[burst] (),
    .\dev_15_req_o_dev_15_req_o[lock] (),
    .\dev_16_req_o_dev_16_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[meta] ),
    .\dev_16_req_o_dev_16_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[addr] ),
    .\dev_16_req_o_dev_16_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[data] ),
    .\dev_16_req_o_dev_16_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[ben] ),
    .\dev_16_req_o_dev_16_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[stb] ),
    .\dev_16_req_o_dev_16_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[rw] ),
    .\dev_16_req_o_dev_16_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[amo] ),
    .\dev_16_req_o_dev_16_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[amoop] ),
    .\dev_16_req_o_dev_16_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[burst] ),
    .\dev_16_req_o_dev_16_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[lock] ),
    .\dev_17_req_o_dev_17_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[meta] ),
    .\dev_17_req_o_dev_17_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[addr] ),
    .\dev_17_req_o_dev_17_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[data] ),
    .\dev_17_req_o_dev_17_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[ben] ),
    .\dev_17_req_o_dev_17_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[stb] ),
    .\dev_17_req_o_dev_17_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[rw] ),
    .\dev_17_req_o_dev_17_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[amo] ),
    .\dev_17_req_o_dev_17_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[amoop] ),
    .\dev_17_req_o_dev_17_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[burst] ),
    .\dev_17_req_o_dev_17_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[lock] ),
    .\dev_18_req_o_dev_18_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[meta] ),
    .\dev_18_req_o_dev_18_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[addr] ),
    .\dev_18_req_o_dev_18_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[data] ),
    .\dev_18_req_o_dev_18_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[ben] ),
    .\dev_18_req_o_dev_18_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[stb] ),
    .\dev_18_req_o_dev_18_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[rw] ),
    .\dev_18_req_o_dev_18_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[amo] ),
    .\dev_18_req_o_dev_18_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[amoop] ),
    .\dev_18_req_o_dev_18_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[burst] ),
    .\dev_18_req_o_dev_18_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[lock] ),
    .\dev_19_req_o_dev_19_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[meta] ),
    .\dev_19_req_o_dev_19_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[addr] ),
    .\dev_19_req_o_dev_19_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[data] ),
    .\dev_19_req_o_dev_19_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[ben] ),
    .\dev_19_req_o_dev_19_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[stb] ),
    .\dev_19_req_o_dev_19_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[rw] ),
    .\dev_19_req_o_dev_19_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[amo] ),
    .\dev_19_req_o_dev_19_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[amoop] ),
    .\dev_19_req_o_dev_19_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[burst] ),
    .\dev_19_req_o_dev_19_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[lock] ),
    .\dev_20_req_o_dev_20_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[meta] ),
    .\dev_20_req_o_dev_20_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[addr] ),
    .\dev_20_req_o_dev_20_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[data] ),
    .\dev_20_req_o_dev_20_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[ben] ),
    .\dev_20_req_o_dev_20_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[stb] ),
    .\dev_20_req_o_dev_20_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[rw] ),
    .\dev_20_req_o_dev_20_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[amo] ),
    .\dev_20_req_o_dev_20_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[amoop] ),
    .\dev_20_req_o_dev_20_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[burst] ),
    .\dev_20_req_o_dev_20_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[lock] ),
    .\dev_21_req_o_dev_21_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[meta] ),
    .\dev_21_req_o_dev_21_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[addr] ),
    .\dev_21_req_o_dev_21_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[data] ),
    .\dev_21_req_o_dev_21_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[ben] ),
    .\dev_21_req_o_dev_21_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[stb] ),
    .\dev_21_req_o_dev_21_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[rw] ),
    .\dev_21_req_o_dev_21_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[amo] ),
    .\dev_21_req_o_dev_21_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[amoop] ),
    .\dev_21_req_o_dev_21_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[burst] ),
    .\dev_21_req_o_dev_21_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[lock] ),
    .\dev_22_req_o_dev_22_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[meta] ),
    .\dev_22_req_o_dev_22_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[addr] ),
    .\dev_22_req_o_dev_22_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[data] ),
    .\dev_22_req_o_dev_22_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[ben] ),
    .\dev_22_req_o_dev_22_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[stb] ),
    .\dev_22_req_o_dev_22_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[rw] ),
    .\dev_22_req_o_dev_22_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[amo] ),
    .\dev_22_req_o_dev_22_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[amoop] ),
    .\dev_22_req_o_dev_22_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[burst] ),
    .\dev_22_req_o_dev_22_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[lock] ),
    .\dev_23_req_o_dev_23_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[meta] ),
    .\dev_23_req_o_dev_23_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[addr] ),
    .\dev_23_req_o_dev_23_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[data] ),
    .\dev_23_req_o_dev_23_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[ben] ),
    .\dev_23_req_o_dev_23_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[stb] ),
    .\dev_23_req_o_dev_23_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[rw] ),
    .\dev_23_req_o_dev_23_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[amo] ),
    .\dev_23_req_o_dev_23_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[amoop] ),
    .\dev_23_req_o_dev_23_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[burst] ),
    .\dev_23_req_o_dev_23_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[lock] ),
    .\dev_24_req_o_dev_24_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[meta] ),
    .\dev_24_req_o_dev_24_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[addr] ),
    .\dev_24_req_o_dev_24_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[data] ),
    .\dev_24_req_o_dev_24_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[ben] ),
    .\dev_24_req_o_dev_24_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[stb] ),
    .\dev_24_req_o_dev_24_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[rw] ),
    .\dev_24_req_o_dev_24_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[amo] ),
    .\dev_24_req_o_dev_24_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[amoop] ),
    .\dev_24_req_o_dev_24_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[burst] ),
    .\dev_24_req_o_dev_24_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[lock] ),
    .\dev_25_req_o_dev_25_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[meta] ),
    .\dev_25_req_o_dev_25_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[addr] ),
    .\dev_25_req_o_dev_25_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[data] ),
    .\dev_25_req_o_dev_25_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[ben] ),
    .\dev_25_req_o_dev_25_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[stb] ),
    .\dev_25_req_o_dev_25_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[rw] ),
    .\dev_25_req_o_dev_25_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[amo] ),
    .\dev_25_req_o_dev_25_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[amoop] ),
    .\dev_25_req_o_dev_25_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[burst] ),
    .\dev_25_req_o_dev_25_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[lock] ),
    .\dev_26_req_o_dev_26_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[meta] ),
    .\dev_26_req_o_dev_26_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[addr] ),
    .\dev_26_req_o_dev_26_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[data] ),
    .\dev_26_req_o_dev_26_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[ben] ),
    .\dev_26_req_o_dev_26_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[stb] ),
    .\dev_26_req_o_dev_26_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[rw] ),
    .\dev_26_req_o_dev_26_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[amo] ),
    .\dev_26_req_o_dev_26_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[amoop] ),
    .\dev_26_req_o_dev_26_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[burst] ),
    .\dev_26_req_o_dev_26_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[lock] ),
    .\dev_27_req_o_dev_27_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[meta] ),
    .\dev_27_req_o_dev_27_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[addr] ),
    .\dev_27_req_o_dev_27_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[data] ),
    .\dev_27_req_o_dev_27_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[ben] ),
    .\dev_27_req_o_dev_27_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[stb] ),
    .\dev_27_req_o_dev_27_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[rw] ),
    .\dev_27_req_o_dev_27_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[amo] ),
    .\dev_27_req_o_dev_27_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[amoop] ),
    .\dev_27_req_o_dev_27_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[burst] ),
    .\dev_27_req_o_dev_27_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[lock] ),
    .\dev_28_req_o_dev_28_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[meta] ),
    .\dev_28_req_o_dev_28_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[addr] ),
    .\dev_28_req_o_dev_28_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[data] ),
    .\dev_28_req_o_dev_28_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[ben] ),
    .\dev_28_req_o_dev_28_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[stb] ),
    .\dev_28_req_o_dev_28_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[rw] ),
    .\dev_28_req_o_dev_28_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[amo] ),
    .\dev_28_req_o_dev_28_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[amoop] ),
    .\dev_28_req_o_dev_28_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[burst] ),
    .\dev_28_req_o_dev_28_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[lock] ),
    .\dev_29_req_o_dev_29_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[meta] ),
    .\dev_29_req_o_dev_29_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[addr] ),
    .\dev_29_req_o_dev_29_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[data] ),
    .\dev_29_req_o_dev_29_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[ben] ),
    .\dev_29_req_o_dev_29_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[stb] ),
    .\dev_29_req_o_dev_29_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[rw] ),
    .\dev_29_req_o_dev_29_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[amo] ),
    .\dev_29_req_o_dev_29_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[amoop] ),
    .\dev_29_req_o_dev_29_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[burst] ),
    .\dev_29_req_o_dev_29_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[lock] ),
    .\dev_30_req_o_dev_30_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[meta] ),
    .\dev_30_req_o_dev_30_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[addr] ),
    .\dev_30_req_o_dev_30_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[data] ),
    .\dev_30_req_o_dev_30_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[ben] ),
    .\dev_30_req_o_dev_30_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[stb] ),
    .\dev_30_req_o_dev_30_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[rw] ),
    .\dev_30_req_o_dev_30_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[amo] ),
    .\dev_30_req_o_dev_30_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[amoop] ),
    .\dev_30_req_o_dev_30_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[burst] ),
    .\dev_30_req_o_dev_30_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[lock] ),
    .\dev_31_req_o_dev_31_req_o[meta] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[meta] ),
    .\dev_31_req_o_dev_31_req_o[addr] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[addr] ),
    .\dev_31_req_o_dev_31_req_o[data] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[data] ),
    .\dev_31_req_o_dev_31_req_o[ben] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[ben] ),
    .\dev_31_req_o_dev_31_req_o[stb] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[stb] ),
    .\dev_31_req_o_dev_31_req_o[rw] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[rw] ),
    .\dev_31_req_o_dev_31_req_o[amo] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[amo] ),
    .\dev_31_req_o_dev_31_req_o[amoop] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[amoop] ),
    .\dev_31_req_o_dev_31_req_o[burst] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[burst] ),
    .\dev_31_req_o_dev_31_req_o[lock] (\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[lock] ));
  assign n381 = io_req[4:0]; // extract
  assign n382 = io_req[36:5]; // extract
  assign n383 = io_req[68:37]; // extract
  assign n384 = io_req[72:69]; // extract
  assign n385 = io_req[73]; // extract
  assign n386 = io_req[74]; // extract
  assign n387 = io_req[75]; // extract
  assign n388 = io_req[79:76]; // extract
  assign n389 = io_req[80]; // extract
  assign n390 = io_req[81]; // extract
  assign n391 = {\io_system_neorv32_bus_io_switch_inst.main_rsp_o_main_rsp_o[data] , \io_system_neorv32_bus_io_switch_inst.main_rsp_o_main_rsp_o[err] , \io_system_neorv32_bus_io_switch_inst.main_rsp_o_main_rsp_o[ack] };
  assign n393 = {\io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_00_req_o_dev_00_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1008:74  */
  assign n395 = iodev_rsp[713:680]; // extract
  assign n396 = n395[0]; // extract
  assign n397 = n395[1]; // extract
  assign n398 = n395[33:2]; // extract
  assign n400 = n322[0]; // extract
  assign n401 = n322[1]; // extract
  assign n402 = n322[33:2]; // extract
  assign n404 = n322[0]; // extract
  assign n405 = n322[1]; // extract
  assign n406 = n322[33:2]; // extract
  assign n408 = n322[0]; // extract
  assign n409 = n322[1]; // extract
  assign n410 = n322[33:2]; // extract
  assign n412 = n322[0]; // extract
  assign n413 = n322[1]; // extract
  assign n414 = n322[33:2]; // extract
  assign n416 = n322[0]; // extract
  assign n417 = n322[1]; // extract
  assign n418 = n322[33:2]; // extract
  assign n420 = n322[0]; // extract
  assign n421 = n322[1]; // extract
  assign n422 = n322[33:2]; // extract
  assign n424 = n322[0]; // extract
  assign n425 = n322[1]; // extract
  assign n426 = n322[33:2]; // extract
  assign n428 = n322[0]; // extract
  assign n429 = n322[1]; // extract
  assign n430 = n322[33:2]; // extract
  assign n432 = n322[0]; // extract
  assign n433 = n322[1]; // extract
  assign n434 = n322[33:2]; // extract
  assign n435 = {\io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_10_req_o_dev_10_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1018:74  */
  assign n437 = iodev_rsp[67:34]; // extract
  assign n438 = n437[0]; // extract
  assign n439 = n437[1]; // extract
  assign n440 = n437[33:2]; // extract
  assign n441 = {\io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_11_req_o_dev_11_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1019:74  */
  assign n443 = iodev_rsp[101:68]; // extract
  assign n444 = n443[0]; // extract
  assign n445 = n443[1]; // extract
  assign n446 = n443[33:2]; // extract
  assign n447 = {\io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_12_req_o_dev_12_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1020:74  */
  assign n449 = iodev_rsp[135:102]; // extract
  assign n450 = n449[0]; // extract
  assign n451 = n449[1]; // extract
  assign n452 = n449[33:2]; // extract
  assign n453 = {\io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_13_req_o_dev_13_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1021:74  */
  assign n455 = iodev_rsp[169:136]; // extract
  assign n456 = n455[0]; // extract
  assign n457 = n455[1]; // extract
  assign n458 = n455[33:2]; // extract
  assign n460 = n322[0]; // extract
  assign n461 = n322[1]; // extract
  assign n462 = n322[33:2]; // extract
  assign n464 = n322[0]; // extract
  assign n465 = n322[1]; // extract
  assign n466 = n322[33:2]; // extract
  assign n467 = {\io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_16_req_o_dev_16_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1024:74  */
  assign n469 = iodev_rsp[203:170]; // extract
  assign n470 = n469[0]; // extract
  assign n471 = n469[1]; // extract
  assign n472 = n469[33:2]; // extract
  assign n473 = {\io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_17_req_o_dev_17_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1025:74  */
  assign n475 = iodev_rsp[237:204]; // extract
  assign n476 = n475[0]; // extract
  assign n477 = n475[1]; // extract
  assign n478 = n475[33:2]; // extract
  assign n479 = {\io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_18_req_o_dev_18_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1026:74  */
  assign n481 = iodev_rsp[271:238]; // extract
  assign n482 = n481[0]; // extract
  assign n483 = n481[1]; // extract
  assign n484 = n481[33:2]; // extract
  assign n485 = {\io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_19_req_o_dev_19_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1027:74  */
  assign n487 = iodev_rsp[33:0]; // extract
  assign n488 = n487[0]; // extract
  assign n489 = n487[1]; // extract
  assign n490 = n487[33:2]; // extract
  assign n491 = {\io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_20_req_o_dev_20_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1028:74  */
  assign n493 = iodev_rsp[305:272]; // extract
  assign n494 = n493[0]; // extract
  assign n495 = n493[1]; // extract
  assign n496 = n493[33:2]; // extract
  assign n497 = {\io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_21_req_o_dev_21_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1029:74  */
  assign n499 = iodev_rsp[339:306]; // extract
  assign n500 = n499[0]; // extract
  assign n501 = n499[1]; // extract
  assign n502 = n499[33:2]; // extract
  assign n503 = {\io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_22_req_o_dev_22_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1030:74  */
  assign n505 = iodev_rsp[373:340]; // extract
  assign n506 = n505[0]; // extract
  assign n507 = n505[1]; // extract
  assign n508 = n505[33:2]; // extract
  assign n509 = {\io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_23_req_o_dev_23_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1031:74  */
  assign n511 = iodev_rsp[407:374]; // extract
  assign n512 = n511[0]; // extract
  assign n513 = n511[1]; // extract
  assign n514 = n511[33:2]; // extract
  assign n515 = {\io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_24_req_o_dev_24_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1032:74  */
  assign n517 = iodev_rsp[441:408]; // extract
  assign n518 = n517[0]; // extract
  assign n519 = n517[1]; // extract
  assign n520 = n517[33:2]; // extract
  assign n521 = {\io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_25_req_o_dev_25_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1033:74  */
  assign n523 = iodev_rsp[475:442]; // extract
  assign n524 = n523[0]; // extract
  assign n525 = n523[1]; // extract
  assign n526 = n523[33:2]; // extract
  assign n527 = {\io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_26_req_o_dev_26_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1034:74  */
  assign n529 = iodev_rsp[509:476]; // extract
  assign n530 = n529[0]; // extract
  assign n531 = n529[1]; // extract
  assign n532 = n529[33:2]; // extract
  assign n533 = {\io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_27_req_o_dev_27_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1035:74  */
  assign n535 = iodev_rsp[543:510]; // extract
  assign n536 = n535[0]; // extract
  assign n537 = n535[1]; // extract
  assign n538 = n535[33:2]; // extract
  assign n539 = {\io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_28_req_o_dev_28_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1036:74  */
  assign n541 = iodev_rsp[577:544]; // extract
  assign n542 = n541[0]; // extract
  assign n543 = n541[1]; // extract
  assign n544 = n541[33:2]; // extract
  assign n545 = {\io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_29_req_o_dev_29_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1037:74  */
  assign n547 = iodev_rsp[611:578]; // extract
  assign n548 = n547[0]; // extract
  assign n549 = n547[1]; // extract
  assign n550 = n547[33:2]; // extract
  assign n551 = {\io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_30_req_o_dev_30_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1038:74  */
  assign n553 = iodev_rsp[645:612]; // extract
  assign n554 = n553[0]; // extract
  assign n555 = n553[1]; // extract
  assign n556 = n553[33:2]; // extract
  assign n557 = {\io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[lock] , \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[burst] , \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[amoop] , \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[amo] , \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[rw] , \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[stb] , \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[ben] , \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[data] , \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[addr] , \io_system_neorv32_bus_io_switch_inst.dev_31_req_o_dev_31_req_o[meta] };
  /* ../../rtl/core/neorv32_top.vhd:1039:74  */
  assign n559 = iodev_rsp[679:646]; // extract
  assign n560 = n559[0]; // extract
  assign n561 = n559[1]; // extract
  assign n562 = n559[33:2]; // extract
  /* ../../rtl/core/neorv32_top.vhd:1167:7  */
  neorv32_clint_1 io_system_neorv32_clint_enabled_neorv32_clint_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_sys),
    .\bus_req_i_bus_req_i[meta] (n572),
    .\bus_req_i_bus_req_i[addr] (n573),
    .\bus_req_i_bus_req_i[data] (n574),
    .\bus_req_i_bus_req_i[ben] (n575),
    .\bus_req_i_bus_req_i[stb] (n576),
    .\bus_req_i_bus_req_i[rw] (n577),
    .\bus_req_i_bus_req_i[amo] (n578),
    .\bus_req_i_bus_req_i[amoop] (n579),
    .\bus_req_i_bus_req_i[burst] (n580),
    .\bus_req_i_bus_req_i[lock] (n581),
    .\bus_rsp_o_bus_rsp_o[ack] (\io_system_neorv32_clint_enabled_neorv32_clint_inst.bus_rsp_o_bus_rsp_o[ack] ),
    .\bus_rsp_o_bus_rsp_o[err] (\io_system_neorv32_clint_enabled_neorv32_clint_inst.bus_rsp_o_bus_rsp_o[err] ),
    .\bus_rsp_o_bus_rsp_o[data] (\io_system_neorv32_clint_enabled_neorv32_clint_inst.bus_rsp_o_bus_rsp_o[data] ),
    .time_o(mtime),
    .mti_o(mti),
    .msi_o(msi));
  /* ../../rtl/core/neorv32_top.vhd:1174:31  */
  assign n571 = iodev_req[737:656]; // extract
  assign n572 = n571[4:0]; // extract
  assign n573 = n571[36:5]; // extract
  assign n574 = n571[68:37]; // extract
  assign n575 = n571[72:69]; // extract
  assign n576 = n571[73]; // extract
  assign n577 = n571[74]; // extract
  assign n578 = n571[75]; // extract
  assign n579 = n571[79:76]; // extract
  assign n580 = n571[80]; // extract
  assign n581 = n571[81]; // extract
  assign n582 = {\io_system_neorv32_clint_enabled_neorv32_clint_inst.bus_rsp_o_bus_rsp_o[data] , \io_system_neorv32_clint_enabled_neorv32_clint_inst.bus_rsp_o_bus_rsp_o[err] , \io_system_neorv32_clint_enabled_neorv32_clint_inst.bus_rsp_o_bus_rsp_o[ack] };
  /* ../../rtl/core/neorv32_top.vhd:1184:20  */
  assign n588 = ~rstn_i;
  /* ../../rtl/core/neorv32_top.vhd:1187:41  */
  assign n590 = mtime[31:0]; // extract
  /* ../../rtl/core/neorv32_top.vhd:1190:28  */
  assign n595 = mtime[63:32]; // extract
  /* ../../rtl/core/neorv32_top.vhd:1190:43  */
  assign n596 = {n595, mtime_lo};
  /* ../../rtl/core/neorv32_top.vhd:1551:5  */
  neorv32_sysinfo_16_0_1_100000000_1_16384_8192_4_4_64_8880fc9ba9bd7b048f24af91b57aaee3b939071a io_system_neorv32_sysinfo_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_sys),
    .\bus_req_i_bus_req_i[meta] (n627),
    .\bus_req_i_bus_req_i[addr] (n628),
    .\bus_req_i_bus_req_i[data] (n629),
    .\bus_req_i_bus_req_i[ben] (n630),
    .\bus_req_i_bus_req_i[stb] (n631),
    .\bus_req_i_bus_req_i[rw] (n632),
    .\bus_req_i_bus_req_i[amo] (n633),
    .\bus_req_i_bus_req_i[amoop] (n634),
    .\bus_req_i_bus_req_i[burst] (n635),
    .\bus_req_i_bus_req_i[lock] (n636),
    .\bus_rsp_o_bus_rsp_o[ack] (\io_system_neorv32_sysinfo_inst.bus_rsp_o_bus_rsp_o[ack] ),
    .\bus_rsp_o_bus_rsp_o[err] (\io_system_neorv32_sysinfo_inst.bus_rsp_o_bus_rsp_o[err] ),
    .\bus_rsp_o_bus_rsp_o[data] (\io_system_neorv32_sysinfo_inst.bus_rsp_o_bus_rsp_o[data] ));
  /* ../../rtl/core/neorv32_top.vhd:1595:29  */
  assign n626 = iodev_req[1557:1476]; // extract
  assign n627 = n626[4:0]; // extract
  assign n628 = n626[36:5]; // extract
  assign n629 = n626[68:37]; // extract
  assign n630 = n626[72:69]; // extract
  assign n631 = n626[73]; // extract
  assign n632 = n626[74]; // extract
  assign n633 = n626[75]; // extract
  assign n634 = n626[79:76]; // extract
  assign n635 = n626[80]; // extract
  assign n636 = n626[81]; // extract
  assign n637 = {\io_system_neorv32_sysinfo_inst.bus_rsp_o_bus_rsp_o[data] , \io_system_neorv32_sysinfo_inst.bus_rsp_o_bus_rsp_o[err] , \io_system_neorv32_sysinfo_inst.bus_rsp_o_bus_rsp_o[ack] };
  assign n643 = {n393, n557, n551, n545, n539, n533, n527, n521, n515, n509, n503, n497, n491, n479, n473, n467, n453, n447, n441, n435, n485};
  assign n644 = {34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, n637, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, n582, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000, 34'b0000000000000000000000000000000000};
  assign n645 = {1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0, 1'b0};
  assign n646 = {n276, n275, n274, n273, n272, n271, n270, n269, n268, n267, n266, n265, n264, n263, n262, 1'b0};
  /* ../../rtl/core/neorv32_top.vhd:1186:9  */
  always @(posedge clk_i or posedge n588)
    if (n588)
      n647 <= 32'b00000000000000000000000000000000;
    else
      n647 <= n590;
endmodule

module neorv32_minimal_wrapper
  (input  clk_i,
   input  rstn_i,
   input  [31:0] xbus_dat_i,
   input  xbus_ack_i,
   output [31:0] xbus_adr_o,
   output [31:0] xbus_dat_o,
   output xbus_we_o,
   output [3:0] xbus_sel_o,
   output xbus_stb_o,
   output xbus_cyc_o,
   output trap_o);
  wire [461:0] trace;
  wire n7;
  wire [461:0] neorv32_top_inst_n10;
  localparam n12 = 1'b0;
  localparam n13 = 1'b0;
  localparam n15 = 1'b0;
  wire [31:0] neorv32_top_inst_n16;
  wire [31:0] neorv32_top_inst_n17;
  wire neorv32_top_inst_n20;
  wire [3:0] neorv32_top_inst_n21;
  wire neorv32_top_inst_n22;
  wire neorv32_top_inst_n23;
  localparam n24 = 1'b0;
  localparam [31:0] n25 = 32'b00000000000000000000000000000000;
  localparam [3:0] n26 = 4'b0000;
  localparam n27 = 1'b0;
  localparam n28 = 1'b0;
  localparam n34 = 1'b0;
  localparam [31:0] n37 = 32'b00000000000000000000000000000000;
  localparam n39 = 1'b0;
  localparam n41 = 1'b0;
  localparam n43 = 1'b0;
  localparam n45 = 1'b0;
  localparam n48 = 1'b0;
  localparam n50 = 1'b0;
  localparam n52 = 1'b0;
  localparam n53 = 1'b1;
  localparam n54 = 1'b1;
  localparam n56 = 1'b1;
  localparam n58 = 1'b1;
  localparam n60 = 1'b1;
  localparam n61 = 1'b1;
  localparam [255:0] n64 = 256'b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;
  localparam n68 = 1'b0;
  localparam n69 = 1'b0;
  localparam n70 = 1'b0;
  wire \neorv32_top_inst.rstn_ocd_o ;
  wire \neorv32_top_inst.rstn_wdt_o ;
  wire \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[valid] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[order] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[insn] ;
  wire \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[trap] ;
  wire \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[halt] ;
  wire \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[intr] ;
  wire [1:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mode] ;
  wire [1:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[ixl] ;
  wire \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[debug] ;
  wire \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[compr] ;
  wire \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[delta] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[cmd32] ;
  wire [4:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs1_addr] ;
  wire [4:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs2_addr] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs1_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs2_rdata] ;
  wire [4:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rd_addr] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rd_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[pc_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[pc_wdata] ;
  wire [11:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[csr_addr] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[csr_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[csr_wdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_addr] ;
  wire [3:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_rmask] ;
  wire [3:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_wmask] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_wdata] ;
  wire \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[valid] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[order] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[insn] ;
  wire \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[trap] ;
  wire \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[halt] ;
  wire \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[intr] ;
  wire [1:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[mode] ;
  wire [1:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[ixl] ;
  wire \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[debug] ;
  wire \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[compr] ;
  wire \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[delta] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[cmd32] ;
  wire [4:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[rs1_addr] ;
  wire [4:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[rs2_addr] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[rs1_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[rs2_rdata] ;
  wire [4:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[rd_addr] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[rd_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[pc_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[pc_wdata] ;
  wire [11:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[csr_addr] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[csr_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[csr_wdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[mem_addr] ;
  wire [3:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[mem_rmask] ;
  wire [3:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[mem_wmask] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[mem_rdata] ;
  wire [31:0] \neorv32_top_inst.trace_cpu1_o_trace_cpu1_o[mem_wdata] ;
  wire \neorv32_top_inst.jtag_tdo_o ;
  wire [2:0] \neorv32_top_inst.xbus_cti_o ;
  wire [2:0] \neorv32_top_inst.xbus_tag_o ;
  wire \neorv32_top_inst.slink_rx_rdy_o ;
  wire [31:0] \neorv32_top_inst.slink_tx_dat_o ;
  wire [3:0] \neorv32_top_inst.slink_tx_dst_o ;
  wire \neorv32_top_inst.slink_tx_val_o ;
  wire \neorv32_top_inst.slink_tx_lst_o ;
  wire [31:0] \neorv32_top_inst.gpio_dir_o ;
  wire [31:0] \neorv32_top_inst.gpio_o ;
  wire \neorv32_top_inst.uart0_txd_o ;
  wire \neorv32_top_inst.uart0_rtsn_o ;
  wire \neorv32_top_inst.uart1_txd_o ;
  wire \neorv32_top_inst.uart1_rtsn_o ;
  wire \neorv32_top_inst.spi_clk_o ;
  wire \neorv32_top_inst.spi_dat_o ;
  wire [7:0] \neorv32_top_inst.spi_csn_o ;
  wire \neorv32_top_inst.sdi_dat_o ;
  wire \neorv32_top_inst.twi_sda_o ;
  wire \neorv32_top_inst.twi_scl_o ;
  wire \neorv32_top_inst.twd_sda_o ;
  wire \neorv32_top_inst.onewire_o ;
  wire [31:0] \neorv32_top_inst.pwm_o ;
  wire [255:0] \neorv32_top_inst.cfs_out_o ;
  wire \neorv32_top_inst.neoled_o ;
  wire [63:0] \neorv32_top_inst.mtime_time_o ;
  wire [461:0] n73;
  assign xbus_adr_o = neorv32_top_inst_n16; //(module output)
  assign xbus_dat_o = neorv32_top_inst_n17; //(module output)
  assign xbus_we_o = neorv32_top_inst_n20; //(module output)
  assign xbus_sel_o = neorv32_top_inst_n21; //(module output)
  assign xbus_stb_o = neorv32_top_inst_n22; //(module output)
  assign xbus_cyc_o = neorv32_top_inst_n23; //(module output)
  assign trap_o = n7; //(module output)
  /* neorv32_minimal_wrapper.vhd:41:10  */
  assign trace = neorv32_top_inst_n10; // (signal)
  /* neorv32_minimal_wrapper.vhd:45:19  */
  assign n7 = trace[67]; // extract
  /* neorv32_minimal_wrapper.vhd:80:23  */
  assign neorv32_top_inst_n10 = n73; // (signal)
  /* neorv32_minimal_wrapper.vhd:47:3  */
  neorv32_top_100000000_1_0_0_0_4_0_64_16384_8192_4_4_64_0_0_1_1_1_1_1_1_1_1_1_0_1_3_5_64_1_0_1_4_1_1_1_8434e1b66992c70a3dedd746c2310434f0b34b57 neorv32_top_inst (
    .clk_i(clk_i),
    .rstn_i(rstn_i),
    .jtag_tck_i(n12),
    .jtag_tdi_i(n13),
    .jtag_tms_i(n15),
    .xbus_dat_i(xbus_dat_i),
    .xbus_ack_i(xbus_ack_i),
    .xbus_err_i(n24),
    .slink_rx_dat_i(n25),
    .slink_rx_src_i(n26),
    .slink_rx_val_i(n27),
    .slink_rx_lst_i(n28),
    .slink_tx_rdy_i(n34),
    .gpio_i(n37),
    .uart0_rxd_i(n39),
    .uart0_ctsn_i(n41),
    .uart1_rxd_i(n43),
    .uart1_ctsn_i(n45),
    .spi_dat_i(n48),
    .sdi_clk_i(n50),
    .sdi_dat_i(n52),
    .sdi_csn_i(n53),
    .twi_sda_i(n54),
    .twi_scl_i(n56),
    .twd_sda_i(n58),
    .twd_scl_i(n60),
    .onewire_i(n61),
    .cfs_in_i(n64),
    .irq_msi_i(n68),
    .irq_mti_i(n69),
    .irq_mei_i(n70),
    .rstn_ocd_o(),
    .rstn_wdt_o(),
    .\trace_cpu0_o_trace_cpu0_o[valid] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[valid] ),
    .\trace_cpu0_o_trace_cpu0_o[order] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[order] ),
    .\trace_cpu0_o_trace_cpu0_o[insn] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[insn] ),
    .\trace_cpu0_o_trace_cpu0_o[trap] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[trap] ),
    .\trace_cpu0_o_trace_cpu0_o[halt] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[halt] ),
    .\trace_cpu0_o_trace_cpu0_o[intr] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[intr] ),
    .\trace_cpu0_o_trace_cpu0_o[mode] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mode] ),
    .\trace_cpu0_o_trace_cpu0_o[ixl] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[ixl] ),
    .\trace_cpu0_o_trace_cpu0_o[debug] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[debug] ),
    .\trace_cpu0_o_trace_cpu0_o[compr] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[compr] ),
    .\trace_cpu0_o_trace_cpu0_o[delta] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[delta] ),
    .\trace_cpu0_o_trace_cpu0_o[cmd32] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[cmd32] ),
    .\trace_cpu0_o_trace_cpu0_o[rs1_addr] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs1_addr] ),
    .\trace_cpu0_o_trace_cpu0_o[rs2_addr] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs2_addr] ),
    .\trace_cpu0_o_trace_cpu0_o[rs1_rdata] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs1_rdata] ),
    .\trace_cpu0_o_trace_cpu0_o[rs2_rdata] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs2_rdata] ),
    .\trace_cpu0_o_trace_cpu0_o[rd_addr] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rd_addr] ),
    .\trace_cpu0_o_trace_cpu0_o[rd_rdata] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rd_rdata] ),
    .\trace_cpu0_o_trace_cpu0_o[pc_rdata] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[pc_rdata] ),
    .\trace_cpu0_o_trace_cpu0_o[pc_wdata] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[pc_wdata] ),
    .\trace_cpu0_o_trace_cpu0_o[csr_addr] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[csr_addr] ),
    .\trace_cpu0_o_trace_cpu0_o[csr_rdata] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[csr_rdata] ),
    .\trace_cpu0_o_trace_cpu0_o[csr_wdata] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[csr_wdata] ),
    .\trace_cpu0_o_trace_cpu0_o[mem_addr] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_addr] ),
    .\trace_cpu0_o_trace_cpu0_o[mem_rmask] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_rmask] ),
    .\trace_cpu0_o_trace_cpu0_o[mem_wmask] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_wmask] ),
    .\trace_cpu0_o_trace_cpu0_o[mem_rdata] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_rdata] ),
    .\trace_cpu0_o_trace_cpu0_o[mem_wdata] (\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_wdata] ),
    .\trace_cpu1_o_trace_cpu1_o[valid] (),
    .\trace_cpu1_o_trace_cpu1_o[order] (),
    .\trace_cpu1_o_trace_cpu1_o[insn] (),
    .\trace_cpu1_o_trace_cpu1_o[trap] (),
    .\trace_cpu1_o_trace_cpu1_o[halt] (),
    .\trace_cpu1_o_trace_cpu1_o[intr] (),
    .\trace_cpu1_o_trace_cpu1_o[mode] (),
    .\trace_cpu1_o_trace_cpu1_o[ixl] (),
    .\trace_cpu1_o_trace_cpu1_o[debug] (),
    .\trace_cpu1_o_trace_cpu1_o[compr] (),
    .\trace_cpu1_o_trace_cpu1_o[delta] (),
    .\trace_cpu1_o_trace_cpu1_o[cmd32] (),
    .\trace_cpu1_o_trace_cpu1_o[rs1_addr] (),
    .\trace_cpu1_o_trace_cpu1_o[rs2_addr] (),
    .\trace_cpu1_o_trace_cpu1_o[rs1_rdata] (),
    .\trace_cpu1_o_trace_cpu1_o[rs2_rdata] (),
    .\trace_cpu1_o_trace_cpu1_o[rd_addr] (),
    .\trace_cpu1_o_trace_cpu1_o[rd_rdata] (),
    .\trace_cpu1_o_trace_cpu1_o[pc_rdata] (),
    .\trace_cpu1_o_trace_cpu1_o[pc_wdata] (),
    .\trace_cpu1_o_trace_cpu1_o[csr_addr] (),
    .\trace_cpu1_o_trace_cpu1_o[csr_rdata] (),
    .\trace_cpu1_o_trace_cpu1_o[csr_wdata] (),
    .\trace_cpu1_o_trace_cpu1_o[mem_addr] (),
    .\trace_cpu1_o_trace_cpu1_o[mem_rmask] (),
    .\trace_cpu1_o_trace_cpu1_o[mem_wmask] (),
    .\trace_cpu1_o_trace_cpu1_o[mem_rdata] (),
    .\trace_cpu1_o_trace_cpu1_o[mem_wdata] (),
    .jtag_tdo_o(),
    .xbus_adr_o(neorv32_top_inst_n16),
    .xbus_dat_o(neorv32_top_inst_n17),
    .xbus_cti_o(),
    .xbus_tag_o(),
    .xbus_we_o(neorv32_top_inst_n20),
    .xbus_sel_o(neorv32_top_inst_n21),
    .xbus_stb_o(neorv32_top_inst_n22),
    .xbus_cyc_o(neorv32_top_inst_n23),
    .slink_rx_rdy_o(),
    .slink_tx_dat_o(),
    .slink_tx_dst_o(),
    .slink_tx_val_o(),
    .slink_tx_lst_o(),
    .gpio_dir_o(),
    .gpio_o(),
    .uart0_txd_o(),
    .uart0_rtsn_o(),
    .uart1_txd_o(),
    .uart1_rtsn_o(),
    .spi_clk_o(),
    .spi_dat_o(),
    .spi_csn_o(),
    .sdi_dat_o(),
    .twi_sda_o(),
    .twi_scl_o(),
    .twd_sda_o(),
    .onewire_o(),
    .pwm_o(),
    .cfs_out_o(),
    .neoled_o(),
    .mtime_time_o());
  assign n73 = {\neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_wdata] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_rdata] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_wmask] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_rmask] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mem_addr] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[csr_wdata] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[csr_rdata] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[csr_addr] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[pc_wdata] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[pc_rdata] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rd_rdata] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rd_addr] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs2_rdata] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs1_rdata] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs2_addr] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[rs1_addr] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[cmd32] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[delta] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[compr] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[debug] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[ixl] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[mode] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[intr] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[halt] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[trap] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[insn] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[order] , \neorv32_top_inst.trace_cpu0_o_trace_cpu0_o[valid] };
endmodule

