module top_module (
    input      cpu_overheated,
    output reg shut_off_computer,
    input      arrived,
    input      gas_tank_empty,
    output reg keep_driving  );

    always @(*) begin
        if (cpu_overheated)
           shut_off_computer = 1;
    end

    always @(*) begin
        if (~arrived)
           keep_driving = ~gas_tank_empty;
    end

endmodule

module tb();
    reg cpu_overheated, arrived, gas_tank_empty;
    wire shut_off_computer, keep_driving;

    top_module m(
        .cpu_overheated(cpu_overheated),
        .shut_off_computer(shut_off_computer),
        .arrived(arrived),
        .gas_tank_empty(gas_tank_empty),
        .keep_driving(keep_driving)
    );

    initial begin
        // @TODO: This is workaround for the fact that we set all values to 0 at the start
        #1 cpu_overheated = 1; arrived = 1; gas_tank_empty = 1;

        #0 cpu_overheated = 0; arrived = 0; gas_tank_empty = 0;
        #1 $vogls_assert_eq(keep_driving, 1);

        // Should latch
        #1 cpu_overheated = 1; arrived = 0; gas_tank_empty = 0;
        #1 $vogls_assert_eq(shut_off_computer, 1);
        #1 cpu_overheated = 0; arrived = 0; gas_tank_empty = 0;
        #1 $vogls_assert_eq(shut_off_computer, 1);

        // Should latch
        #1 cpu_overheated = 0; arrived = 0; gas_tank_empty = 0;
        #1 $vogls_assert_eq(keep_driving, 1);
        #1 cpu_overheated = 0; arrived = 1; gas_tank_empty = 1;
        #1 $vogls_assert_eq(keep_driving, 1);
    end
endmodule
