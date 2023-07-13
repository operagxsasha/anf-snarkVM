// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkVM library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use console::{
    account::{Address, PrivateKey},
    prelude::*,
    program::{Ciphertext, Literal, Plaintext, ProgramOwner, Record},
    types::Field,
};
use ledger_block::{
    Block,
    CompactBatchCertificate,
    CompactBatchHeader,
    ConfirmedTransaction,
    Deployment,
    Execution,
    Fee,
    Header,
    Input,
    Output,
    Transaction,
    Transactions,
    Transition,
    TransmissionID,
};
use ledger_query::Query;
use ledger_store::{helpers::memory::BlockMemory, BlockStore};
use synthesizer_process::Process;
use synthesizer_program::Program;

use indexmap::IndexSet;
use once_cell::sync::OnceCell;

type CurrentNetwork = console::network::Testnet3;
type CurrentAleo = circuit::network::AleoV0;

/****************************************** Transition ********************************************/

/// Samples a random transition.
pub fn sample_transition(rng: &mut TestRng) -> Transition<CurrentNetwork> {
    crate::sample_execution(rng).into_transitions().next().unwrap()
}

/// Sample the transition inputs.
pub fn sample_inputs() -> Vec<(<CurrentNetwork as Network>::TransitionID, Input<CurrentNetwork>)> {
    let rng = &mut TestRng::default();

    // Sample a transition.
    let transaction = crate::sample_execution_transaction_with_fee(rng);
    let transition = transaction.transitions().next().unwrap();

    // Retrieve the transition ID and input.
    let transition_id = *transition.id();
    let input = transition.inputs().iter().next().unwrap().clone();

    // Sample a random plaintext.
    let plaintext = Plaintext::Literal(Literal::Field(Uniform::rand(rng)), Default::default());
    let plaintext_hash = CurrentNetwork::hash_bhp1024(&plaintext.to_bits_le()).unwrap();
    // Sample a random ciphertext.
    let ciphertext = Ciphertext::from_fields(&vec![Uniform::rand(rng); 10]).unwrap();
    let ciphertext_hash = CurrentNetwork::hash_bhp1024(&ciphertext.to_bits_le()).unwrap();

    vec![
        (transition_id, input),
        (Uniform::rand(rng), Input::Constant(Uniform::rand(rng), None)),
        (Uniform::rand(rng), Input::Constant(plaintext_hash, Some(plaintext.clone()))),
        (Uniform::rand(rng), Input::Public(Uniform::rand(rng), None)),
        (Uniform::rand(rng), Input::Public(plaintext_hash, Some(plaintext))),
        (Uniform::rand(rng), Input::Private(Uniform::rand(rng), None)),
        (Uniform::rand(rng), Input::Private(ciphertext_hash, Some(ciphertext))),
        (Uniform::rand(rng), Input::Record(Uniform::rand(rng), Uniform::rand(rng))),
        (Uniform::rand(rng), Input::ExternalRecord(Uniform::rand(rng))),
    ]
}

/// Sample the transition outputs.
pub fn sample_outputs() -> Vec<(<CurrentNetwork as Network>::TransitionID, Output<CurrentNetwork>)> {
    let rng = &mut TestRng::default();

    // Sample a transition.
    let transaction = crate::sample_execution_transaction_with_fee(rng);
    let transition = transaction.transitions().next().unwrap();

    // Retrieve the transition ID and input.
    let transition_id = *transition.id();
    let input = transition.outputs().iter().next().unwrap().clone();

    // Sample a random plaintext.
    let plaintext = Plaintext::Literal(Literal::Field(Uniform::rand(rng)), Default::default());
    let plaintext_hash = CurrentNetwork::hash_bhp1024(&plaintext.to_bits_le()).unwrap();
    // Sample a random ciphertext.
    let ciphertext = Ciphertext::from_fields(&vec![Uniform::rand(rng); 10]).unwrap();
    let ciphertext_hash = CurrentNetwork::hash_bhp1024(&ciphertext.to_bits_le()).unwrap();
    // Sample a random record.
    let randomizer = Uniform::rand(rng);
    let nonce = CurrentNetwork::g_scalar_multiply(&randomizer);
    let record = Record::<CurrentNetwork, Plaintext<CurrentNetwork>>::from_str(
        &format!("{{ owner: aleo1d5hg2z3ma00382pngntdp68e74zv54jdxy249qhaujhks9c72yrs33ddah.private, token_amount: 100u64.private, _nonce: {nonce}.public }}"),
    ).unwrap();
    let record_ciphertext = record.encrypt(randomizer).unwrap();
    let record_checksum = CurrentNetwork::hash_bhp1024(&record_ciphertext.to_bits_le()).unwrap();

    vec![
        (transition_id, input),
        (Uniform::rand(rng), Output::Constant(Uniform::rand(rng), None)),
        (Uniform::rand(rng), Output::Constant(plaintext_hash, Some(plaintext.clone()))),
        (Uniform::rand(rng), Output::Public(Uniform::rand(rng), None)),
        (Uniform::rand(rng), Output::Public(plaintext_hash, Some(plaintext))),
        (Uniform::rand(rng), Output::Private(Uniform::rand(rng), None)),
        (Uniform::rand(rng), Output::Private(ciphertext_hash, Some(ciphertext))),
        (Uniform::rand(rng), Output::Record(Uniform::rand(rng), Uniform::rand(rng), None)),
        (Uniform::rand(rng), Output::Record(Uniform::rand(rng), record_checksum, Some(record_ciphertext))),
        (Uniform::rand(rng), Output::ExternalRecord(Uniform::rand(rng))),
    ]
}

/******************************************* Deployment *******************************************/

pub fn sample_deployment(rng: &mut TestRng) -> Deployment<CurrentNetwork> {
    static INSTANCE: OnceCell<Deployment<CurrentNetwork>> = OnceCell::new();
    INSTANCE
        .get_or_init(|| {
            // Initialize a new program.
            let (string, program) = Program::<CurrentNetwork>::parse(
                r"
program testing.aleo;

mapping store:
    key item as u32.public;
    value object as u32.public;

function compute:
    input r0 as u32.private;
    add r0 r0 into r1;
    output r1 as u32.public;",
            )
            .unwrap();
            assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

            // Construct the process.
            let process = Process::load().unwrap();
            // Compute the deployment.
            let deployment = process.deploy::<CurrentAleo, _>(&program, rng).unwrap();
            // Return the deployment.
            // Note: This is a testing-only hack to adhere to Rust's dependency cycle rules.
            Deployment::from_str(&deployment.to_string()).unwrap()
        })
        .clone()
}

/******************************************* Execution ********************************************/

/// Samples a random execution.
pub fn sample_execution(rng: &mut TestRng) -> Execution<CurrentNetwork> {
    // Sample the genesis block.
    let block = crate::sample_genesis_block(rng);
    // Retrieve a transaction.
    let transaction = block.transactions().iter().next().unwrap().deref().clone();
    // Retrieve the execution.
    if let Transaction::Execute(_, execution, _) = transaction { execution } else { unreachable!() }
}

/********************************************** Fee ***********************************************/

/// Samples a random hardcoded fee.
pub fn sample_fee_hardcoded(rng: &mut TestRng) -> Fee<CurrentNetwork> {
    static INSTANCE: OnceCell<Fee<CurrentNetwork>> = OnceCell::new();
    INSTANCE
        .get_or_init(|| {
            // Sample a deployment or execution ID.
            let deployment_or_execution_id = Field::rand(rng);
            // Sample a fee.
            sample_fee(deployment_or_execution_id, rng)
        })
        .clone()
}

/// Samples a random fee.
pub fn sample_fee(deployment_or_execution_id: Field<CurrentNetwork>, rng: &mut TestRng) -> Fee<CurrentNetwork> {
    // Sample the genesis block, transaction, and private key.
    let (block, transaction, private_key) = crate::sample_genesis_block_and_components(rng);
    // Retrieve a credits record.
    let credits = transaction.records().next().unwrap().1.clone();
    // Decrypt the record.
    let credits = credits.decrypt(&private_key.try_into().unwrap()).unwrap();
    // Set the fee amount.
    let fee = 10_000_000;

    // Initialize the process.
    let process = Process::load().unwrap();
    // Compute the fee trace.
    let (_, _, mut trace) =
        process.execute_fee::<CurrentAleo, _>(&private_key, credits, fee, deployment_or_execution_id, rng).unwrap();

    // Initialize a new block store.
    let block_store = BlockStore::<CurrentNetwork, BlockMemory<_>>::open(None).unwrap();
    // Insert the block into the block store.
    // Note: This is a testing-only hack to adhere to Rust's dependency cycle rules.
    block_store.insert(&FromStr::from_str(&block.to_string()).unwrap()).unwrap();

    // Prepare the assignments.
    trace.prepare(Query::from(block_store)).unwrap();
    // Compute the proof and construct the fee.
    let fee = trace.prove_fee::<CurrentAleo, _>(rng).unwrap();

    // Convert the fee.
    // Note: This is a testing-only hack to adhere to Rust's dependency cycle rules.
    Fee::from_str(&fee.to_string()).unwrap()
}

/****************************************** Transaction *******************************************/

/// Samples a random deployment transaction.
pub fn sample_deployment_transaction(rng: &mut TestRng) -> Transaction<CurrentNetwork> {
    // Sample a private key.
    let private_key = PrivateKey::new(rng).unwrap();
    // Sample a deployment.
    let deployment = crate::sample_deployment(rng);

    // Compute the deployment ID.
    let deployment_id = deployment.to_deployment_id().unwrap();
    // Construct a program owner.
    let owner = ProgramOwner::new(&private_key, deployment_id, rng).unwrap();

    // Sample the fee.
    let fee = crate::sample_fee(deployment_id, rng);

    // Construct a deployment transaction.
    Transaction::from_deployment(owner, deployment, fee).unwrap()
}

/// Samples a random execution transaction with fee.
pub fn sample_execution_transaction_with_fee(rng: &mut TestRng) -> Transaction<CurrentNetwork> {
    // Sample an execution.
    let execution = crate::sample_execution(rng);
    // Compute the execution ID.
    let execution_id = execution.to_execution_id().unwrap();

    // Sample the fee.
    let fee = crate::sample_fee(execution_id, rng);

    // Construct an execution transaction.
    Transaction::from_execution(execution, Some(fee)).unwrap()
}

/// Samples a random fee transaction.
pub fn sample_fee_transaction(rng: &mut TestRng) -> Transaction<CurrentNetwork> {
    // Sample a fee.
    let fee = crate::sample_fee_hardcoded(rng);
    // Construct a fee transaction.
    Transaction::from_fee(fee).unwrap()
}

/****************************************** Transactions ******************************************/

/// Samples a block transactions.
pub fn sample_block_transactions(rng: &mut TestRng) -> Transactions<CurrentNetwork> {
    crate::sample_genesis_block(rng).transactions().clone()
}

/********************************************* Block **********************************************/

/// Samples a random genesis block.
pub fn sample_genesis_block(rng: &mut TestRng) -> Block<CurrentNetwork> {
    // Sample the genesis block and components.
    let (block, _, _) = crate::sample_genesis_block_and_components(rng);
    // Return the block.
    block
}

/// Samples a random genesis block and the transaction from the genesis block.
pub fn sample_genesis_block_and_transaction(rng: &mut TestRng) -> (Block<CurrentNetwork>, Transaction<CurrentNetwork>) {
    // Sample the genesis block and components.
    let (block, transaction, _) = crate::sample_genesis_block_and_components(rng);
    // Return the block and transaction.
    (block, transaction)
}

/// Samples a random genesis block, the transaction from the genesis block, and the genesis private key.
pub fn sample_genesis_block_and_components(
    rng: &mut TestRng,
) -> (Block<CurrentNetwork>, Transaction<CurrentNetwork>, PrivateKey<CurrentNetwork>) {
    static INSTANCE: OnceCell<(Block<CurrentNetwork>, Transaction<CurrentNetwork>, PrivateKey<CurrentNetwork>)> =
        OnceCell::new();
    INSTANCE.get_or_init(|| crate::sample_genesis_block_and_components_raw(rng)).clone()
}

/// Samples a random genesis block, the transaction from the genesis block, and the genesis private key.
fn sample_genesis_block_and_components_raw(
    rng: &mut TestRng,
) -> (Block<CurrentNetwork>, Transaction<CurrentNetwork>, PrivateKey<CurrentNetwork>) {
    // Sample the genesis private key.
    let private_key = PrivateKey::new(rng).unwrap();
    let address = Address::<CurrentNetwork>::try_from(private_key).unwrap();

    // Prepare the locator.
    let locator = ("credits.aleo", "mint");
    // Prepare the amount for each call to the mint function.
    let amount = 100_000_000u64;
    // Prepare the function inputs.
    let inputs = [address.to_string(), format!("{amount}_u64")];

    // Initialize the process.
    let process = Process::load().unwrap();
    // Authorize the mint function.
    let authorization =
        process.authorize::<CurrentAleo, _>(&private_key, locator.0, locator.1, inputs.iter(), rng).unwrap();
    // Execute the mint function.
    let (_, mut trace) = process.execute::<CurrentAleo>(authorization).unwrap();

    // Initialize a new block store.
    let block_store = BlockStore::<CurrentNetwork, BlockMemory<_>>::open(None).unwrap();

    // Prepare the assignments.
    trace.prepare(Query::from(block_store)).unwrap();
    // Compute the proof and construct the execution.
    let execution = trace.prove_execution::<CurrentAleo, _>(locator.0, rng).unwrap();
    // Convert the execution.
    // Note: This is a testing-only hack to adhere to Rust's dependency cycle rules.
    let execution = Execution::from_str(&execution.to_string()).unwrap();

    // Construct the transaction.
    let transaction = Transaction::from_execution(execution, None).unwrap();
    // Prepare the confirmed transaction.
    let confirmed = ConfirmedTransaction::accepted_execute(0, transaction.clone(), vec![]).unwrap();
    // Prepare the transactions.
    let transactions = Transactions::from_iter([confirmed].into_iter());

    // Prepare the block header.
    let header = Header::genesis(&transactions).unwrap();
    // Prepare the previous block hash.
    let previous_hash = <CurrentNetwork as Network>::BlockHash::default();

    // Prepare the genesis transmission ids.
    let transmission_ids =
        transactions.iter().map(|transaction| TransmissionID::from(&transaction.id())).collect::<IndexSet<_>>();
    // Prepare the initial compact batch header.
    let compact_batch_header =
        CompactBatchHeader::new(&private_key, header.round(), transmission_ids, Default::default(), rng).unwrap();
    // TODO (raychu86): batch - Currently using the same signature as the header. Considering skipping the requirement for the genesis certificate.
    // Prepare the signatures.
    let signatures = [(*compact_batch_header.signature(), compact_batch_header.timestamp())].into();
    // Prepare the initial compact batch certificate.
    let compact_batch_certificate = CompactBatchCertificate::new(compact_batch_header, signatures).unwrap();

    // Construct the block.
    let block =
        Block::new(previous_hash, header, transactions, vec![], None, compact_batch_certificate, Default::default())
            .unwrap();
    assert!(block.header().is_genesis(), "Failed to initialize a genesis block");
    // Return the block, transaction, and private key.
    (block, transaction, private_key)
}
