use crate::test_utils::tests::codegen;

#[test]
fn var_output_in_function_call() {
    let result = codegen(
        r#"FUNCTION func : DINT
            VAR_OUTPUT  o   : INT;      END_VAR
            o := 6;
            func := 4;
        END_FUNCTION

        PROGRAM main
            VAR
                x : INT := 4;
            END_VAR

            func(o => x);
        END_PROGRAM
        "#,
    );

    insta::assert_snapshot!(result);
}

#[test]
#[ignore = "duplicate"]
fn on_functions_var_in_out_should_be_passed_as_a_pointer() {
    let result = codegen(
        r#"
        FUNCTION bump : DINT
            VAR_IN_OUT  v  : SINT;      END_VAR
            bump := v;
            v := 7;
        END_FUNCTION
        "#,
    );

    insta::assert_snapshot!(result);
}

#[test]
fn on_functions_var_output_should_be_passed_as_a_pointer() {
    let result = codegen(
        r#"
        FUNCTION bump : DINT
            VAR_OUTPUT  v  : SINT;      END_VAR
            bump := 1;
            v := 2;
        END_FUNCTION
        "#,
    );

    insta::assert_snapshot!(result);
}

#[test]
fn member_variables_in_body() {
    let result = codegen(
        r#"FUNCTION func : DINT
            VAR_INPUT   i   : INT := 6 END_VAR
            VAR_IN_OUT  io  : SINT;      END_VAR
            VAR_OUTPUT  o   : LINT;      END_VAR
            VAR         v   : INT := 1; END_VAR
            VAR_TEMP    vt  : INT := 2; END_VAR
            
            func := i * io - o + v * vt;
        END_FUNCTION
        "#,
    );

    insta::assert_snapshot!(result);
}

#[test]
fn simple_call() {
    let result = codegen(
        r#"FUNCTION func : DINT
            VAR_INPUT x : DINT; END_VAR
        END_FUNCTION

        PROGRAM main
            VAR a : DINT; END_VAR

            func(a);
            func(1);
            func(1+a);
        END_PROGRAM
        "#,
    );

    insta::assert_snapshot!(result, @r###"
    ; ModuleID = 'main'
    source_filename = "main"

    %main_interface = type { i32 }

    @main_instance = global %main_interface zeroinitializer

    define i32 @func(i32 %0) {
    entry:
      %x = alloca i32, align 4
      store i32 %0, i32* %x, align 4
      %func = alloca i32, align 4
      store i32 0, i32* %func, align 4
      %func_ret = load i32, i32* %func, align 4
      ret i32 %func_ret
    }

    define void @main(%main_interface* %0) {
    entry:
      %a = getelementptr inbounds %main_interface, %main_interface* %0, i32 0, i32 0
      %load_a = load i32, i32* %a, align 4
      %call = call i32 @func(i32 %load_a)
      %call1 = call i32 @func(i32 1)
      %load_a2 = load i32, i32* %a, align 4
      %tmpVar = add i32 1, %load_a2
      %call3 = call i32 @func(i32 %tmpVar)
      ret void
    }
    "###);
}

#[test]
fn passing_a_string_to_a_function() {
    let result = codegen(
        r#"FUNCTION func : DINT
            VAR_INPUT x : STRING[5]; END_VAR
        END_FUNCTION

        PROGRAM main
            VAR a : STRING[5]; END_VAR

            func(a);
            func('12345');
        END_PROGRAM
        "#,
    );

    insta::assert_snapshot!(result, @r###"
    ; ModuleID = 'main'
    source_filename = "main"

    %main_interface = type { [6 x i8] }

    @main_instance = global %main_interface zeroinitializer
    @utf08_literal_0 = unnamed_addr constant [6 x i8] c"12345\00"

    define i32 @func([6 x i8] %0) {
    entry:
      %x = alloca [6 x i8], align 1
      store [6 x i8] %0, [6 x i8]* %x, align 1
      %func = alloca i32, align 4
      store i32 0, i32* %func, align 4
      %func_ret = load i32, i32* %func, align 4
      ret i32 %func_ret
    }

    define void @main(%main_interface* %0) {
    entry:
      %a = getelementptr inbounds %main_interface, %main_interface* %0, i32 0, i32 0
      %1 = alloca [6 x i8], align 1
      %2 = bitcast [6 x i8]* %1 to i8*
      call void @llvm.memset.p0i8.i64(i8* align 1 %2, i8 0, i64 ptrtoint ([6 x i8]* getelementptr ([6 x i8], [6 x i8]* null, i32 1) to i64), i1 false)
      %3 = bitcast [6 x i8]* %1 to i8*
      %4 = bitcast [6 x i8]* %a to i8*
      call void @llvm.memcpy.p0i8.p0i8.i32(i8* align 1 %3, i8* align 1 %4, i32 5, i1 false)
      %5 = load [6 x i8], [6 x i8]* %1, align 1
      %call = call i32 @func([6 x i8] %5)
      %6 = alloca [6 x i8], align 1
      %7 = bitcast [6 x i8]* %6 to i8*
      call void @llvm.memset.p0i8.i64(i8* align 1 %7, i8 0, i64 ptrtoint ([6 x i8]* getelementptr ([6 x i8], [6 x i8]* null, i32 1) to i64), i1 false)
      %8 = bitcast [6 x i8]* %6 to i8*
      call void @llvm.memcpy.p0i8.p0i8.i32(i8* align 1 %8, i8* align 1 getelementptr inbounds ([6 x i8], [6 x i8]* @utf08_literal_0, i32 0, i32 0), i32 5, i1 false)
      %9 = load [6 x i8], [6 x i8]* %6, align 1
      %call1 = call i32 @func([6 x i8] %9)
      ret void
    }

    ; Function Attrs: argmemonly nofree nounwind willreturn writeonly
    declare void @llvm.memset.p0i8.i64(i8* nocapture writeonly, i8, i64, i1 immarg) #0

    ; Function Attrs: argmemonly nofree nounwind willreturn
    declare void @llvm.memcpy.p0i8.p0i8.i32(i8* noalias nocapture writeonly, i8* noalias nocapture readonly, i32, i1 immarg) #1

    attributes #0 = { argmemonly nofree nounwind willreturn writeonly }
    attributes #1 = { argmemonly nofree nounwind willreturn }
    "###);
}