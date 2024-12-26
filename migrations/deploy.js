const anchor = require('@project-serum/anchor');
const fs = require('fs');
const path = require('path');

async function deploy() {
  const provider = anchor.Provider.local();
  anchor.setProvider(provider);

  // Load the program's keypair from file
  const programKeypair = anchor.web3.Keypair.generate();

  // Load the IDL (Interface Definition Language) of the program
  const idl = JSON.parse(fs.readFileSync(path.resolve(__dirname, './target/idl/solana_staking_program.json')));

  // Deploy the program
  const program = await anchor.workspace.SolanaStakingProgram.deploy({
    programKeypair,
    idl,
    provider,
  });

  console.log('Program deployed to:', program.programId.toString());
}

deploy().catch(err => {
  console.error(err);
});
