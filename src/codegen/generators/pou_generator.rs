/// Copyright (c) 2020 Ghaith Hachem and Mathias Rieder

/// The pou_generator contains functions to generate the code for POUs (PROGRAM, FUNCTION, FUNCTION_BLOCK)
/// # responsibilities
/// - generates a struct-datatype for the POU's members
/// - generates a function for the pou
/// - declares a global instance if the POU is a PROGRAM
use crate::index::ImplementationIndexEntry;
use inkwell::types::StructType;
use super::{expression_generator::ExpressionCodeGenerator, llvm::LLVM, statement_generator::{FunctionContext, StatementCodeGenerator}};
use crate::codegen::llvm_index::LLVMTypedIndex;
use crate::typesystem::*;
use crate::{ast::{DataTypeDeclaration, POU, Implementation, PouType, SourceRange, Statement, Variable, VariableBlock, VariableBlockType}, compile_error::CompileError, index::GlobalIndex};
use inkwell::{AddressSpace, module::Module, types::{BasicTypeEnum, FunctionType}, values::{BasicValueEnum, FunctionValue}};


pub struct PouGenerator<'ink, 'cg> {
    llvm: LLVM<'ink>,
    global_index : &'cg GlobalIndex, 
    index: &'cg LLVMTypedIndex<'ink>,
}

/// Creates opaque implementations for all callable items in the index 
/// Returns a Typed index containing the associated implementations.
pub fn generate_implementation_stubs<'ink>(module : &Module<'ink>, llvm : LLVM<'ink>, global_index : &GlobalIndex, types_index : &LLVMTypedIndex<'ink>) -> Result<LLVMTypedIndex<'ink>, CompileError> {
    let mut index = LLVMTypedIndex::new();
    let pou_generator = PouGenerator::new(llvm, global_index, &types_index);
    for (name, implementation) in global_index.get_implementations() {
        let curr_f = pou_generator.generate_implementation_stub(implementation, module)?;
        index
            .associate_implementation(name, curr_f)?;
    }

    Ok(index)
}

impl<'ink,'cg> PouGenerator<'ink,'cg> {

    /// creates a new PouGenerator
    ///
    /// the PouGenerator needs a mutable index to register the generated pou
    pub fn new(
        llvm: LLVM<'ink>,
        global_index : &'cg GlobalIndex, 
        index: &'cg LLVMTypedIndex<'ink>,
    ) -> PouGenerator<'ink, 'cg> {
        PouGenerator {
            llvm,
            global_index,
            index,
        }
    }

    // /// generates the stub of a POU (The function call placehoder, as well as the associated
    // /// struct
    // pub fn generate_pou_stub(&self, pou: &POU,module: &Module<'a>) -> Result<LLVMTypedIndex, CompileError> {
    //     let index = LLVMTypedIndex::new(self.index);
    //     let pou_name = &pou.name;

    //     //generate the instance-struct type
    //     let pou_members: Vec<&Variable> = pou
    //         .variable_blocks
    //         .iter()
    //         .flat_map(|it| it.variables.iter())
    //         .collect();
    //     let instance_struct_type = {
    //         let mut struct_generator =
    //             StructGenerator::new(self.llvm, self.index);
    //         let (struct_type, initial_value) = struct_generator.generate_struct_type(&pou_members, pou_name)?;
    //         self.index.associate_type_initial_value(pou_name, initial_value);
    //         struct_type
    //     };


    //     //generate a global variable if it's a program
    //     if pou.pou_type == PouType::Program {
    //         let instance_initializer = index
    //             .find_type_initial_value(pou_name);
    //         let global_value = self.llvm.create_global_variable(
    //                 module, 
    //                 &struct_generator::get_pou_instance_variable_name(pou_name),
    //                 instance_struct_type.into(),
    //                 instance_initializer);
    //         index
    //             .associate_global_variable(pou_name, global_value.as_pointer_value());
    //     }

    //     let struct_name = format!("{}_interface", pou_name);
    //     let struct_type = self.llvm.context.opaque_struct_type(struct_name.as_str());
    //     index.associate_type(
    //         pou_name,
    //         struct_type.into(),
    //     );
    //     Ok(index)

    // }

    pub fn generate_implementation_stub(&self, implementation : &ImplementationIndexEntry, module : &Module<'ink>) -> Result<FunctionValue<'ink>, CompileError> {
        let global_index = self.global_index;
        //generate a function that takes a instance-struct parameter
        let pou_name = implementation.get_call_name();
        let instance_struct_type : StructType = self.index.get_associated_type(implementation.get_type_name()).map(|it| it.into_struct_type())?;
        let return_type : Option<&DataType> = global_index.find_return_type(implementation.get_type_name()); //TODO find the pou type, find inside the the variable declared as return (if any)
        let return_type = return_type
                .map(DataType::get_name)
                .map(|it|  self.index.get_associated_type(it).unwrap());

        let function_declaration = self.create_llvm_function_type(
            vec![instance_struct_type.ptr_type(AddressSpace::Generic).into()],
            return_type,
        );

        let curr_f = module.add_function(pou_name, function_declaration, None);
        Ok(curr_f)
    }

    /// generates a function for the given pou
    pub fn generate_pou(&self, pou: &POU, implementation : &Implementation) -> Result<(), CompileError> {

        let context = self.llvm.context;
        let mut local_index = LLVMTypedIndex::create_child(self.index);

        let return_type = pou
            .return_type
            .as_ref()
            .and_then(DataTypeDeclaration::get_name)
            .and_then(|it| self.index.find_associated_type(it));

        let pou_name = &pou.name;

        let pou_members: Vec<&Variable> = pou
            .variable_blocks
            .iter()
            .flat_map(|it| it.variables.iter())
            .collect();


        //TODO : Index local variables


        let current_function = self.index.find_associated_implementation(pou_name)
            .ok_or_else(|| CompileError::codegen_error(format!("Could not find generated stub for {}",pou_name), pou.location.clone()))?;
        
        //generate the body
        let block = context.append_basic_block(current_function, "entry");
        self.llvm.builder.position_at_end(block);
        
        //generate the return-variable
        if let Some(return_type) = return_type {
            let return_variable = self.llvm.create_local_variable(pou_name, &return_type);
            local_index.associate_loded_local_variable(pou_name, pou_name, return_variable)?;
        }

        // generate loads for all the parameters
        self.generate_local_variable_accessors(
            &mut local_index,
            pou_name,
            current_function,
            &pou_members,
        )?;

        let function_context = FunctionContext{linking_context: pou_name.clone(), function: current_function};
        {
            let statement_gen = StatementCodeGenerator::new(&self.llvm, self.global_index, &local_index, &function_context);
            //if this is a function, we need to initilialize the VAR-variables
            if pou.pou_type == PouType::Function {
                self.generate_initialization_of_local_vars(&pou.variable_blocks, &statement_gen)?;
            }
            statement_gen.generate_body(&implementation.statements)?
        }

        // generate return statement
        self.generate_return_statement(&function_context, &local_index, pou.pou_type, Some(0..0))?; //TODO location

        Ok(())
    }

    /// TODO llvm.rs
    /// generates a llvm `FunctionType` that takes the given list of `parameters` and
    /// returns the given `return_type`
    fn create_llvm_function_type(
        &self,
        parameters: Vec<BasicTypeEnum<'ink>>,
        return_type: Option<BasicTypeEnum<'ink>>,
    ) -> FunctionType<'ink> {
        let params = parameters.as_slice();
        match return_type {
            Some(enum_type) if enum_type.is_int_type() => {
                enum_type.into_int_type().fn_type(params, false)
            }
            Some(enum_type) if enum_type.is_float_type() => {
                enum_type.into_float_type().fn_type(params, false)
            }
            Some(enum_type) if enum_type.is_array_type() => {
                enum_type.into_array_type().fn_type(params, false)
            }
            None => self.llvm.context.void_type().fn_type(params, false),
            _ => panic!(format!("Unsupported return type {:?}", return_type)),
        }
    }

    /// generates a load-statement for the given member
    fn generate_local_variable_accessors(
        &self,
        index : &mut LLVMTypedIndex<'ink>,
        pou_name: &str,
        current_function: FunctionValue<'ink>,
        members: &Vec<&Variable>
    ) -> Result<(), CompileError> {

        //Generate reference to parameter
        for (i, m) in members.iter().enumerate() {
            let parameter_name = &m.name;

            let ptr_value = current_function
                .get_first_param()
                .map(BasicValueEnum::into_pointer_value)
                .ok_or_else(|| CompileError::MissingFunctionError{location: m.location.clone()})?;

            index.associate_loded_local_variable(
                pou_name,
                parameter_name,
                self.llvm.builder
                    .build_struct_gep(ptr_value, i as u32, &parameter_name)
                    .unwrap(),
            )?;
        }

        Ok(())
    }

    /// generates assignment statements for initialized variables in the VAR-block
    ///
    /// - `blocks` - all declaration blocks of the current pou
    fn generate_initialization_of_local_vars(
        &self,
        blocks : &Vec<VariableBlock>,
        statement_generator: &StatementCodeGenerator<'ink, '_>,
    )-> Result<(), CompileError> {
        let variables_with_initializers = blocks.iter()
            .filter(|it| it.variable_block_type == VariableBlockType::Local)
            .flat_map(|it| &it.variables)
            .filter(|it| it.initializer.is_some());

        for variable in variables_with_initializers {
            let left = Statement::Reference{ name: variable.name.clone(), location: variable.location.clone() };
            let right = variable.initializer.as_ref().unwrap();
            statement_generator.generate_assignment_statement(&left, right)?;
        }
        Ok(())
    }

    /// generates the function's return statement only if the given pou_type is a `PouType::Function`
    ///
    /// a function returns the value of the local variable that has the function's name
    fn generate_return_statement(&self, function_context: &FunctionContext<'ink>, local_index : &LLVMTypedIndex<'ink>, pou_type: PouType, location: Option<SourceRange>) -> Result<(), CompileError> {
        match pou_type {
            PouType::Function => {
                let reference = Statement::Reference{
                    name: function_context.linking_context.clone(),
                    location: location.unwrap_or(0usize..0usize)
                };
                let mut exp_gen = ExpressionCodeGenerator::new(&self.llvm, self.global_index , local_index, None, &function_context);
                exp_gen.temp_variable_prefix = "".to_string();
                exp_gen.temp_variable_suffix = "_ret".to_string();
                let (_, value) = exp_gen.generate_expression(&reference)?;
                self.llvm.builder.build_return(Some(&value));
            }
            _ => {
                self.llvm.builder.build_return(None);
            }
        }
        Ok(())
    }
}


