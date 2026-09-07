## Build Program

```bash
$ cargo risczero build --manifest-path program_methods/guest/Cargo.toml
Building guest package: program_methods
Docker context: "/home/namauser/logos-ecosystem/project/e-identity-stack"
Building ELF binaries in program_methods for riscv32im-risc0-zkvm-elf target...
Docker version 29.7.2, build a7dcaa6fdb
Cargo.lock not found in path /home/namauser/logos-ecosystem/project/e-identity-stack/program_methods/guest/Cargo.lock
[+] Building 243.6s (10/10) FINISHED                                                            docker:default
 => [internal] load build definition from Dockerfile                                                      0.2s
 => => transferring dockerfile: 980B                                                                      0.0s
 => [internal] load metadata for docker.io/risczero/risc0-guest-builder:r0.1.91.1                         1.9s
 => [build 1/5] FROM docker.io/risczero/risc0-guest-builder:r0.1.91.1@sha256:fafb377a44e1cfca415577c48d2  0.0s
 => [internal] load build context                                                                         0.2s
 => => transferring context: 47.12kB                                                                      0.0s
 => CACHED [build 2/5] WORKDIR /src                                                                       0.0s
 => [build 3/5] COPY . .                                                                                  0.3s
 => [build 4/5] RUN cargo +risc0 fetch --locked --target riscv32im-risc0-zkvm-elf --manifest-path progr  56.8s
 => [build 5/5] RUN cargo +risc0 build --release --locked --target riscv32im-risc0-zkvm-elf --manifest  170.1s 
 => [export 1/1] COPY --from=build /src/target/riscv32im-risc0-zkvm-elf/release /                         5.3s 
 => exporting to client directory                                                                         0.9s 
 => => copying files 188.31MB                                                                             0.9s 
                                                                                                               
 2 warnings found (use docker --debug to expand):                                                              
 - FromAsCasing: 'as' and 'FROM' keywords' casing do not match (line 14)                                       
 - FromAsCasing: 'as' and 'FROM' keywords' casing do not match (line 1)
ELFs ready at:
ImageID: b8edb92e7e3eca8a9210609edac675a388a7f69256a8f0b28b9404f1a1cbe12a - /home/namauser/logos-ecosystem/project/e-identity-stack/target/riscv32im-risc0-zkvm-elf/docker/forum_membership_proof.bin
ImageID: 45fcc43f74bca2614b417fe39559ac0c4cccdae0092187223d8cfc2e98ed7c16 - /home/namauser/logos-ecosystem/project/e-identity-stack/target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
```

## Deploy Program

```bash
$ wallet deploy-program target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
Transaction hash is e8af9dc3af21d369a26b9a494eef274d5b5a0720ca722c036d76965fe84a4889
Transaction is included in block 41478
Transaction data is ProgramDeployment(ProgramDeploymentTransaction { message: Message { bytecode: <554536 bytes> } })
Stored persistent accounts at /home/namauser/.lee/wallet/storage.json
Stored statistics at /home/namauser/.lee/wallet/statistics.json
```

## Variable Environment

```bash
$ export PROG=target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
export MY_ACCOUNT=Public/9p7BZn9g6UrVMBiatyeNtq4yv9DitxYM1ZXsjYi6vf47
export FORUM_ID=0302030405060708091011121314151617181920212223242526272829303132
export COMMITMENT=0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435
export ROOM_ID=4387847341ba4199b858e82e6d2114e8ef00611ae38bfce0ead60983e2fda6ca
export MOD_PUBKEY=aa01020304050607080910111213141516171819202122232425262728293031
```

## Initialize Forum
```bash
$ spel --idl idl.json -p $PROG -- \
  initialize-forum \
  --admin $MY_ACCOUNT \
  --forum-id $FORUM_ID \
  --k-strikes 3 --n-moderators 2 --m-moderators 3
📋 Instruction: initialize_forum

Accounts:
  📦 state → 6GmmipskJUQzCstnyPDWrM9uGp98cXAENwtR6yoRUQQa (PDA)
    seeds: [program_id, "forum", Arg(forum_id)]
  📦 admin → 0x82eec89bab481a5d70bb753ab162ae0f53dfcb761f35fd20c210911e926b6f04

Arguments (parsed):
  forum_id = 0x0302030405060708091011121314151617181920212223242526272829303132
  k_strikes = 3
  n_moderators = 2
  m_moderators = 3

🔧 Transaction:
  program-id: 45fcc43f74bca2614b417fe39559ac0c4cccdae0092187223d8cfc2e98ed7c16
  program:    target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
  instruction index: 0
  instruction: InitializeForum {
    forum_id: 0x0302030405060708091011121314151617181920212223242526272829303132,
    k_strikes: 3,
    n_moderators: 2,
    m_moderators: 3,
  }

  Serialized instruction data (36 u32 words):
    [00000000, 00000003, 00000002, 00000003, 00000004, 00000005, 00000006, 00000007, 00000008, 00000009, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000003, 00000002, 00000003]

📤 Submitting transaction...
📤 Transaction submitted!
   tx_hash: 0336928961511b5d1a5ad50519978155c03824c4eb2832e6b4a5330d0b59da24
   Waiting for confirmation...
✅ Transaction confirmed — included in a block.
```

## Register Member
```bash
$ spel --idl idl.json -p $PROG -- \
  register-member \
  --member $MY_ACCOUNT \
  --forum-id $FORUM_ID \
  --commitment $COMMITMENT \
  --stake-amount 1000
📋 Instruction: register_member

Accounts:
  📦 state → 6GmmipskJUQzCstnyPDWrM9uGp98cXAENwtR6yoRUQQa (PDA)
    seeds: [program_id, "forum", Arg(forum_id)]
  📦 member → 0x82eec89bab481a5d70bb753ab162ae0f53dfcb761f35fd20c210911e926b6f04

Arguments (parsed):
  forum_id = 0x0302030405060708091011121314151617181920212223242526272829303132
  commitment = 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435
  stake_amount = 1000

🔧 Transaction:
  program-id: 45fcc43f74bca2614b417fe39559ac0c4cccdae0092187223d8cfc2e98ed7c16
  program:    target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
  instruction index: 1
  instruction: RegisterMember {
    forum_id: 0x0302030405060708091011121314151617181920212223242526272829303132,
    commitment: 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
    stake_amount: 1000,
  }

  Serialized instruction data (67 u32 words):
    [00000001, 00000003, 00000002, 00000003, 00000004, 00000005, 00000006, 00000007, 00000008, 00000009, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 0000000a, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035, 000003e8, 00000000]

📤 Submitting transaction...
📤 Transaction submitted!
   tx_hash: 2bcae58f49a5029a4bef4b55a6194d92147718ac739262a61fc56799962f5020
   Waiting for confirmation...
✅ Transaction confirmed — included in a block.
```

## Register Room  
```bash
$ spel --idl idl.json -p $PROG -- \
  register-room \
  --admin $MY_ACCOUNT \
  --forum-id $FORUM_ID \
  --admin-commitment $COMMITMENT \
  --n-mod-threshold 1 --m-mod-total 1 \
  --moderator-pubkeys $MOD_PUBKEY \
  --min-members-for-maturity 1
📋 Instruction: register_room

Accounts:
  📦 state → 6GmmipskJUQzCstnyPDWrM9uGp98cXAENwtR6yoRUQQa (PDA)
    seeds: [program_id, "forum", Arg(forum_id)]
  📦 admin → 0x82eec89bab481a5d70bb753ab162ae0f53dfcb761f35fd20c210911e926b6f04

Arguments (parsed):
  forum_id = 0x0302030405060708091011121314151617181920212223242526272829303132
  admin_commitment = 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435
  n_mod_threshold = 1
  m_mod_total = 1
  moderator_pubkeys = [0xaa01020304050607080910111213141516171819202122232425262728293031]
  min_members_for_maturity = 1

🔧 Transaction:
  program-id: 45fcc43f74bca2614b417fe39559ac0c4cccdae0092187223d8cfc2e98ed7c16
  program:    target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
  instruction index: 3
  instruction: RegisterRoom {
    forum_id: 0x0302030405060708091011121314151617181920212223242526272829303132,
    admin_commitment: 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
    n_mod_threshold: 1,
    m_mod_total: 1,
    moderator_pubkeys: [0xaa01020304050607080910111213141516171819202122232425262728293031],
    min_members_for_maturity: 1,
  }

  Serialized instruction data (101 u32 words):
    [00000003, 00000003, 00000002, 00000003, 00000004, 00000005, 00000006, 00000007, 00000008, 00000009, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 0000000a, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035, 00000001, 00000001, 00000001, 000000aa, 00000001, 00000002, 00000003, 00000004, 00000005, 00000006, 00000007, 00000008, 00000009, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000001]

📤 Submitting transaction...
📤 Transaction submitted!
   tx_hash: 921161cbe61eec1581a08d5da48b179d913ed123e23f43cd6af9301f5021e0e0
   Waiting for confirmation...
✅ Transaction confirmed — included in a block.
```

## Join Room
```bash
$ spel --idl idl.json -p $PROG -- \
  join-room \
  --member $MY_ACCOUNT \
  --forum-id $FORUM_ID \
  --room-id $ROOM_ID \
  --member-commitment $COMMITMENT
📋 Instruction: join_room

Accounts:
  📦 state → 6GmmipskJUQzCstnyPDWrM9uGp98cXAENwtR6yoRUQQa (PDA)
    seeds: [program_id, "forum", Arg(forum_id)]
  📦 member → 0x82eec89bab481a5d70bb753ab162ae0f53dfcb761f35fd20c210911e926b6f04

Arguments (parsed):
  forum_id = 0x0302030405060708091011121314151617181920212223242526272829303132
  room_id = 0x4387847341ba4199b858e82e6d2114e8ef00611ae38bfce0ead60983e2fda6ca
  member_commitment = 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435

🔧 Transaction:
  program-id: 45fcc43f74bca2614b417fe39559ac0c4cccdae0092187223d8cfc2e98ed7c16
  program:    target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
  instruction index: 2
  instruction: JoinRoom {
    forum_id: 0x0302030405060708091011121314151617181920212223242526272829303132,
    room_id: 0x4387847341ba4199b858e82e6d2114e8ef00611ae38bfce0ead60983e2fda6ca,
    member_commitment: 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
  }

  Serialized instruction data (97 u32 words):
    [00000002, 00000003, 00000002, 00000003, 00000004, 00000005, 00000006, 00000007, 00000008, 00000009, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000043, 00000087, 00000084, 00000073, 00000041, 000000ba, 00000041, 00000099, 000000b8, 00000058, 000000e8, 0000002e, 0000006d, 00000021, 00000014, 000000e8, 000000ef, 00000000, 00000061, 0000001a, 000000e3, 0000008b, 000000fc, 000000e0, 000000ea, 000000d6, 00000009, 00000083, 000000e2, 000000fd, 000000a6, 000000ca, 0000000a, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035]

📤 Submitting transaction...
📤 Transaction submitted!
   tx_hash: 253204c6f610ea7e5b0e5f41925fd672cb488139a6620e72c8ef186e0cae663c
   Waiting for confirmation...
✅ Transaction confirmed — included in a block.
```

## Record Strike

### 1st Strike

```bash
$ spel --idl idl.json -p $PROG -- \
  record-strike \
  --forum-id $FORUM_ID \
  --room-id $ROOM_ID \
  --target-commitment $COMMITMENT \
  --evidence-hash bb0b0c0d0e0f1011121314151617181920212223242526272829303132333435 \
  --n-valid-sigs 1
📋 Instruction: record_strike

Accounts:
  📦 state → 6GmmipskJUQzCstnyPDWrM9uGp98cXAENwtR6yoRUQQa (PDA)
    seeds: [program_id, "forum", Arg(forum_id)]

Arguments (parsed):
  forum_id = 0x0302030405060708091011121314151617181920212223242526272829303132
  room_id = 0x4387847341ba4199b858e82e6d2114e8ef00611ae38bfce0ead60983e2fda6ca
  target_commitment = 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435
  evidence_hash = 0xbb0b0c0d0e0f1011121314151617181920212223242526272829303132333435
  n_valid_sigs = 1

🔧 Transaction:
  program-id: 45fcc43f74bca2614b417fe39559ac0c4cccdae0092187223d8cfc2e98ed7c16
  program:    target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
  instruction index: 4
  instruction: RecordStrike {
    forum_id: 0x0302030405060708091011121314151617181920212223242526272829303132,
    room_id: 0x4387847341ba4199b858e82e6d2114e8ef00611ae38bfce0ead60983e2fda6ca,
    target_commitment: 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
    evidence_hash: 0xbb0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
    n_valid_sigs: 1,
  }

  Serialized instruction data (130 u32 words):
    [00000004, 00000003, 00000002, 00000003, 00000004, 00000005, 00000006, 00000007, 00000008, 00000009, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000043, 00000087, 00000084, 00000073, 00000041, 000000ba, 00000041, 00000099, 000000b8, 00000058, 000000e8, 0000002e, 0000006d, 00000021, 00000014, 000000e8, 000000ef, 00000000, 00000061, 0000001a, 000000e3, 0000008b, 000000fc, 000000e0, 000000ea, 000000d6, 00000009, 00000083, 000000e2, 000000fd, 000000a6, 000000ca, 0000000a, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035, 000000bb, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035, 00000001]

📤 Submitting transaction...
📤 Transaction submitted!
   tx_hash: 71bb08574c7cc2ce892f0955842f7219524caab01ce68c727c41fd3cdeed7df7
   Waiting for confirmation...
✅ Transaction confirmed — included in a block.
```

### 2nd Strike

```bash
$ spel --idl idl.json -p $PROG -- \
  record-strike \
  --forum-id $FORUM_ID \
  --room-id $ROOM_ID \
  --target-commitment $COMMITMENT \
  --evidence-hash cc0b0c0d0e0f1011121314151617181920212223242526272829303132333435 \
  --n-valid-sigs 1
📋 Instruction: record_strike

Accounts:
  📦 state → 6GmmipskJUQzCstnyPDWrM9uGp98cXAENwtR6yoRUQQa (PDA)
    seeds: [program_id, "forum", Arg(forum_id)]

Arguments (parsed):
  forum_id = 0x0302030405060708091011121314151617181920212223242526272829303132
  room_id = 0x4387847341ba4199b858e82e6d2114e8ef00611ae38bfce0ead60983e2fda6ca
  target_commitment = 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435
  evidence_hash = 0xcc0b0c0d0e0f1011121314151617181920212223242526272829303132333435
  n_valid_sigs = 1

🔧 Transaction:
  program-id: 45fcc43f74bca2614b417fe39559ac0c4cccdae0092187223d8cfc2e98ed7c16
  program:    target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
  instruction index: 4
  instruction: RecordStrike {
    forum_id: 0x0302030405060708091011121314151617181920212223242526272829303132,
    room_id: 0x4387847341ba4199b858e82e6d2114e8ef00611ae38bfce0ead60983e2fda6ca,
    target_commitment: 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
    evidence_hash: 0xcc0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
    n_valid_sigs: 1,
  }

  Serialized instruction data (130 u32 words):
    [00000004, 00000003, 00000002, 00000003, 00000004, 00000005, 00000006, 00000007, 00000008, 00000009, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000043, 00000087, 00000084, 00000073, 00000041, 000000ba, 00000041, 00000099, 000000b8, 00000058, 000000e8, 0000002e, 0000006d, 00000021, 00000014, 000000e8, 000000ef, 00000000, 00000061, 0000001a, 000000e3, 0000008b, 000000fc, 000000e0, 000000ea, 000000d6, 00000009, 00000083, 000000e2, 000000fd, 000000a6, 000000ca, 0000000a, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035, 000000cc, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035, 00000001]

📤 Submitting transaction...
📤 Transaction submitted!
   tx_hash: d0a0b271abdf92a958508099a60391069ecd78327f32a33e4cba67f1e0b094f7
   Waiting for confirmation...
✅ Transaction confirmed — included in a block.
```

### 3rd Strike
```bash
$ spel --idl idl.json -p $PROG -- \
  record-strike \
  --forum-id $FORUM_ID \
  --room-id $ROOM_ID \
  --target-commitment $COMMITMENT \
  --evidence-hash dd0b0c0d0e0f1011121314151617181920212223242526272829303132333435 \
  --n-valid-sigs 1
📋 Instruction: record_strike

Accounts:
  📦 state → 6GmmipskJUQzCstnyPDWrM9uGp98cXAENwtR6yoRUQQa (PDA)
    seeds: [program_id, "forum", Arg(forum_id)]

Arguments (parsed):
  forum_id = 0x0302030405060708091011121314151617181920212223242526272829303132
  room_id = 0x4387847341ba4199b858e82e6d2114e8ef00611ae38bfce0ead60983e2fda6ca
  target_commitment = 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435
  evidence_hash = 0xdd0b0c0d0e0f1011121314151617181920212223242526272829303132333435
  n_valid_sigs = 1

🔧 Transaction:
  program-id: 45fcc43f74bca2614b417fe39559ac0c4cccdae0092187223d8cfc2e98ed7c16
  program:    target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
  instruction index: 4
  instruction: RecordStrike {
    forum_id: 0x0302030405060708091011121314151617181920212223242526272829303132,
    room_id: 0x4387847341ba4199b858e82e6d2114e8ef00611ae38bfce0ead60983e2fda6ca,
    target_commitment: 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
    evidence_hash: 0xdd0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
    n_valid_sigs: 1,
  }

  Serialized instruction data (130 u32 words):
    [00000004, 00000003, 00000002, 00000003, 00000004, 00000005, 00000006, 00000007, 00000008, 00000009, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000043, 00000087, 00000084, 00000073, 00000041, 000000ba, 00000041, 00000099, 000000b8, 00000058, 000000e8, 0000002e, 0000006d, 00000021, 00000014, 000000e8, 000000ef, 00000000, 00000061, 0000001a, 000000e3, 0000008b, 000000fc, 000000e0, 000000ea, 000000d6, 00000009, 00000083, 000000e2, 000000fd, 000000a6, 000000ca, 0000000a, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035, 000000dd, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035, 00000001]

📤 Submitting transaction...
📤 Transaction submitted!
   tx_hash: 56687a74822b0b397f2852a923f763b327c794cde7680f93b6e78a7f1b807f94
   Waiting for confirmation...
✅ Transaction confirmed — included in a block.
```

## E. Slash Member
```bash
$ spel --idl idl.json -p $PROG -- \
  slash-member \
  --authority $MY_ACCOUNT \
  --forum-id $FORUM_ID \
  --target-commitment $COMMITMENT \
  --k-rooms-min 1 --min-room-age-indexes 0 --min-room-members 0
📋 Instruction: slash_member

Accounts:
  📦 state → 6GmmipskJUQzCstnyPDWrM9uGp98cXAENwtR6yoRUQQa (PDA)
    seeds: [program_id, "forum", Arg(forum_id)]
  📦 authority → 0x82eec89bab481a5d70bb753ab162ae0f53dfcb761f35fd20c210911e926b6f04

Arguments (parsed):
  forum_id = 0x0302030405060708091011121314151617181920212223242526272829303132
  target_commitment = 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435
  k_rooms_min = 1
  min_room_age_indexes = 0
  min_room_members = 0

🔧 Transaction:
  program-id: 45fcc43f74bca2614b417fe39559ac0c4cccdae0092187223d8cfc2e98ed7c16
  program:    target/riscv32im-risc0-zkvm-elf/docker/membership_registry.bin
  instruction index: 6
  instruction: SlashMember {
    forum_id: 0x0302030405060708091011121314151617181920212223242526272829303132,
    target_commitment: 0x0a0b0c0d0e0f1011121314151617181920212223242526272829303132333435,
    k_rooms_min: 1,
    min_room_age_indexes: 0,
    min_room_members: 0,
  }

  Serialized instruction data (69 u32 words):
    [00000006, 00000003, 00000002, 00000003, 00000004, 00000005, 00000006, 00000007, 00000008, 00000009, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 0000000a, 0000000b, 0000000c, 0000000d, 0000000e, 0000000f, 00000010, 00000011, 00000012, 00000013, 00000014, 00000015, 00000016, 00000017, 00000018, 00000019, 00000020, 00000021, 00000022, 00000023, 00000024, 00000025, 00000026, 00000027, 00000028, 00000029, 00000030, 00000031, 00000032, 00000033, 00000034, 00000035, 00000001, 00000000, 00000000, 00000000]

📤 Submitting transaction...
📤 Transaction submitted!
   tx_hash: 8479875cd6e08558bcef27311905fe3db4f594b48d8ffe63538690b2c80421a4
   Waiting for confirmation...
✅ Transaction confirmed — included in a block.
```