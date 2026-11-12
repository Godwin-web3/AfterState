import React, { useEffect, useRef, useState } from 'react';
import { BrowserProvider, JsonRpcProvider, Interface, id, randomBytes, hexlify, parseUnits } from 'ethers';
import { NearConnector } from '@hot-labs/near-connect';
import { bytesToHex, extractEvmSignature, extractNearTxHash, fixedBytes, readAdapterConfig, waitForNearOutcome } from './near';
import { Activity, ArrowDownLeft, ArrowRight, ArrowUpRight, BadgeCheck, Check, CheckCircle2, ChevronDown, CircleAlert, Code2, Copy, ExternalLink, FileCheck2, Fingerprint, Gauge, Layers3, Menu, Plus, RadioTower, RefreshCw, Settings2, Shield, Wallet, X, Zap } from 'lucide-react';
import { CHAINS, DEFAULT_CONFIG, REGISTRY_ABI, SOURCE_ABI, TOKEN_ABI, SETTLEMENT_ABI, compact, validAddress } from './protocol';

const empty = { recipient:'', min:'1', max:'100', deadline:'' };
const read = (key, fallback) => { try { return JSON.parse(localStorage.getItem(key) || 'null') || fallback; } catch { return fallback; } };

export default function App() {
  const [page,setPage] = useState('overview');
  const [config,setConfig] = useState(() => ({...DEFAULT_CONFIG,...read('afterstate:config',{})}));
  const [account,setAccount] = useState('');
  const [chainId,setChainId] = useState(0);
  const [nearAccount,setNearAccount] = useState('');
  const nearConnector = useRef(null);
  const [busy,setBusy] = useState('');
  const [toast,setToast] = useState(null);
  const [form,setForm] = useState(empty);
  const [agreements,setAgreements] = useState(() => read('afterstate:agreements',[]));
  const [events,setEvents] = useState(() => read('afterstate:activity',[]));
  const [network,setNetwork] = useState({owner:'',verifier:'',adapter:'',symbol:'ASD',decimals:6,error:'',loading:false});
  const [mobile,setMobile] = useState(false);

  const sourceOk = validAddress(config.sourceToken) && validAddress(config.sourceEmitter);
  const tokenMatch = validAddress(config.sourceToken) && validAddress(config.destinationToken) && config.sourceToken.toLowerCase() === config.destinationToken.toLowerCase();
  const setupCount = [config.sourceToken,config.sourceEmitter,config.destinationToken,config.destinationRegistry,config.destinationSettlement].filter(validAddress).length;
  const ready = sourceOk && tokenMatch && validAddress(config.destinationRegistry) && validAddress(config.destinationSettlement);
  const notify = (kind,title,message) => { setToast({kind,title,message}); window.clearTimeout(window.__asToast); window.__asToast = window.setTimeout(() => setToast(null),6500); };
  const saveEvents = (next) => { setEvents(next); localStorage.setItem('afterstate:activity',JSON.stringify(next)); };
  const saveAgreements = (next) => { setAgreements(next); localStorage.setItem('afterstate:agreements',JSON.stringify(next)); };
  const addEvent = (event) => saveEvents([{id:hexlify(randomBytes(8)),at:Date.now(),...event},...events].slice(0,50));

  useEffect(() => {
    if (!window.ethereum) return;
    const accounts = (list) => setAccount(list[0] || '');
    const chain = (hex) => setChainId(parseInt(hex,16));
    window.ethereum.on?.('accountsChanged',accounts);
    window.ethereum.on?.('chainChanged',chain);
    return () => { window.ethereum.removeListener?.('accountsChanged',accounts); window.ethereum.removeListener?.('chainChanged',chain); };
  },[]);

  useEffect(() => {
    const connector = new NearConnector({ network:'testnet', features:{ testnet:true, signAndSendTransaction:true }, footerBranding:null });
    nearConnector.current = connector;
    const onSignIn = event => setNearAccount(event?.accounts?.[0]?.accountId || '');
    const onSignOut = () => setNearAccount('');
    connector.on('wallet:signIn', onSignIn);
    connector.on('wallet:signOut', onSignOut);
    return () => {
      connector.off?.('wallet:signIn', onSignIn);
      connector.off?.('wallet:signOut', onSignOut);
    };
  },[]);

  async function connectNear() {
    try {
      await nearConnector.current.connect();
      const wallet = await nearConnector.current.wallet();
      const accounts = await wallet.getAccounts?.();
      if (accounts?.[0]?.accountId) setNearAccount(accounts[0].accountId);
      else if (typeof accounts?.[0] === 'string') setNearAccount(accounts[0]);
    } catch(e) { notify('error','NEAR connection failed',e.message || 'Wallet connection was not completed.'); }
  }

  async function connect() {
    if (!window.ethereum) return notify('warning','Wallet not detected','Open this app inside MetaMask Mobile, Coinbase Wallet, or another EIP-1193 wallet browser.');
    setBusy('connect');
    try {
      const provider = new BrowserProvider(window.ethereum);
      const accounts = await provider.send('eth_requestAccounts',[]);
      const chain = await provider.getNetwork();
      setAccount(accounts[0] || ''); setChainId(Number(chain.chainId));
      notify('success','Wallet connected',compact(accounts[0],6));
    } catch(e) { notify('error','Connection failed',e.shortMessage || e.message); }
    finally { setBusy(''); }
  }

  async function switchTo(which) {
    if (!window.ethereum) throw new Error('Connect an EVM wallet first.');
    const c = CHAINS[which];
    try {
      await window.ethereum.request({method:'wallet_switchEthereumChain',params:[{chainId:'0x'+c.id.toString(16)}]});
    } catch(e) {
      if (e.code !== 4902) throw e;
      await window.ethereum.request({method:'wallet_addEthereumChain',params:[{chainId:'0x'+c.id.toString(16),chainName:c.name,nativeCurrency:{name:'Sepolia Ether',symbol:'ETH',decimals:18},rpcUrls:[c.rpc],blockExplorerUrls:[c.explorer]}]});
    }
    setChainId(c.id);
  }

  async function refreshNetwork() {
    if (!validAddress(config.destinationRegistry)) return setNetwork(n => ({...n,error:'Enter the destination registry address to read live state.',loading:false}));
    setNetwork(n => ({...n,loading:true,error:''}));
    try {
      const p = new JsonRpcProvider(CHAINS.destination.rpc);
      const registry = new (await import('ethers')).Contract(config.destinationRegistry,REGISTRY_ABI,p);
      const [owner,verifier,adapter] = await Promise.all([registry.owner(),registry.verifier(),registry.settlementAdapter()]);
      let symbol = 'Token', decimals = 6;
      if (validAddress(config.destinationToken)) {
        const token = new (await import('ethers')).Contract(config.destinationToken,TOKEN_ABI,p);
        [symbol,decimals] = await Promise.all([token.symbol().catch(()=>'Token'),token.decimals().then(Number).catch(()=>6)]);
      }
      setNetwork({owner,verifier,adapter,symbol,decimals,error:'',loading:false});
    } catch(e) { setNetwork(n => ({...n,loading:false,error:e.shortMessage || e.message || 'RPC read failed'})); }
  }
  useEffect(() => { refreshNetwork(); },[config.destinationRegistry,config.destinationToken]);

  async function createAgreement(e) {
    e.preventDefault();
    if (!account) return notify('warning','Connect wallet','The destination registry owner must sign this transaction.');
    if (!ready) return notify('warning','Deployment configuration incomplete','Configure all contract addresses and use the same token address on both chains.');
    if (!validAddress(form.recipient)) return notify('warning','Invalid recipient','Enter a valid EVM address.');
    if (!form.deadline || new Date(form.deadline).getTime() <= Date.now()) return notify('warning','Invalid deadline','Choose a future deadline.');
    if (Number(form.min)<=0 || Number(form.max)<Number(form.min)) return notify('warning','Invalid amount range','Maximum must be at least the minimum.');
    setBusy('create');
    try {
      await switchTo('destination');
      const p = new BrowserProvider(window.ethereum), signer = await p.getSigner();
      const registry = new (await import('ethers')).Contract(config.destinationRegistry,REGISTRY_ABI,signer);
      const owner = await registry.owner();
      if (owner.toLowerCase() !== account.toLowerCase()) throw new Error('Connected wallet is not the destination registry owner. Owner: '+owner);
      const token = new (await import('ethers')).Contract(config.destinationToken,TOKEN_ABI,p);
      const decimals = Number(await token.decimals());
      const agreementId = hexlify(randomBytes(32));
      const schema = id('SourcePayment(bytes32,uint32,address,address,address,uint256,uint64,bytes32)');
      const deadline = Math.floor(new Date(form.deadline).getTime()/1000);
      const tx = await registry.createAgreement(agreementId,CHAINS.base.id,config.sourceEmitter,schema,config.destinationToken,form.recipient,parseUnits(form.min,decimals),parseUnits(form.max,decimals),deadline);
      notify('info','Registering agreement','Waiting for Ethereum Sepolia confirmation.');
      const receipt = await tx.wait();
      const item = {id:agreementId,recipient:form.recipient,min:form.min,max:form.max,deadline,status:'registered',createdAt:Date.now(),registryTx:receipt.hash,symbol:network.symbol};
      saveAgreements([item,...agreements]);
      addEvent({type:'agreement',title:'Agreement registered',detail:compact(agreementId),hash:receipt.hash,chain:'destination'});
      setForm(empty); setPage('agreements');
      notify('success','Agreement registered','The destination registry accepted the agreement.');
    } catch(e) { notify('error','Registration failed',e.shortMessage || e.reason || e.message); }
    finally { setBusy(''); }
  }

  async function paySource(a) {
    if (!account) return notify('warning','Connect wallet','Connect the payer wallet before paying.');
    if (!sourceOk) return notify('warning','Source contracts missing','Configure the Base Sepolia token and emitter.');
    const value = window.prompt('Payment amount ('+(network.symbol || 'token')+')',a.min);
    if (value === null) return;
    if (!Number.isFinite(Number(value)) || Number(value)<Number(a.min) || Number(value)>Number(a.max)) return notify('warning','Amount outside agreement','Payment must be within '+a.min+'–'+a.max+'.');
    setBusy('pay');
    try {
      await switchTo('base');
      const p = new BrowserProvider(window.ethereum), signer = await p.getSigner();
      const token = new (await import('ethers')).Contract(config.sourceToken,TOKEN_ABI,signer);
      const emitter = new (await import('ethers')).Contract(config.sourceEmitter,SOURCE_ABI,signer);
      const decimals = Number(await token.decimals()), amount = parseUnits(value,decimals);
      if (await token.allowance(account,config.sourceEmitter) < amount) {
        const approval = await token.approve(config.sourceEmitter,amount); notify('info','Approve token spending','Waiting for approval.'); await approval.wait();
      }
      const nonce = hexlify(randomBytes(32));
      const tx = await emitter.pay(a.id,1,config.sourceToken,a.recipient,amount,nonce);
      notify('info','Source payment submitted','Waiting for Base Sepolia confirmation.');
      const receipt = await tx.wait();
      saveAgreements(agreements.map(x => x.id===a.id ? {...x,sourceTx:receipt.hash,nonce,paid:value,status:'source-paid'} : x));
      addEvent({type:'payment',title:'Source payment confirmed',detail:value+' '+(network.symbol || 'ASD')+' · '+compact(a.id),hash:receipt.hash,chain:'base'});
      notify('success','Source payment confirmed','The source payment is final. NEAR MPC verification and destination execution are separate steps.');
    } catch(e) { notify('error','Payment failed',e.shortMessage || e.reason || e.message); }
    finally { setBusy(''); }
  }


  async function verifyWithNear(a) {
    if (!nearAccount) return notify('warning','Connect NEAR wallet','Connect the NEAR testnet account configured as the adapter authorized submitter.');
    if (!account) return notify('warning','Connect EVM wallet','Connect the EVM wallet that will submit the accepted fact on Ethereum Sepolia.');
    if (!a.sourceTx || !a.nonce) return notify('warning','Source payment missing','This agreement has no confirmed source payment.');
    if (!validNearAccountId(config.nearAdapterAccount)) return notify('warning','Invalid adapter account','Enter the deployed NEAR testnet adapter account ID.');
    setBusy('verify');
    try {
      const adapter = config.nearAdapterAccount.trim();
      const adapterConfig = await readAdapterConfig(adapter);
      if (adapterConfig.authorized_submitter !== nearAccount) {
        throw new Error('Connected NEAR account is not the adapter authorized_submitter. Expected ' + adapterConfig.authorized_submitter);
      }
      const asHex = bytes => '0x' + bytes.map(v=>Number(v).toString(16).padStart(2,'0')).join('');
      const sourceEmitter = asHex(adapterConfig.source_emitter);
      const agreementId = asHex(adapterConfig.agreement_id);
      const asset = asHex(adapterConfig.asset);
      const recipient = asHex(adapterConfig.recipient);
      if (sourceEmitter.toLowerCase() !== config.sourceEmitter.toLowerCase()
        || agreementId.toLowerCase() !== a.id.toLowerCase()
        || asset.toLowerCase() !== config.sourceToken.toLowerCase()
        || recipient.toLowerCase() !== a.recipient.toLowerCase()
        || Number(adapterConfig.agreement_version) !== 1
        || Number(adapterConfig.destination_chain_id) !== CHAINS.destination.id
        || asHex(adapterConfig.destination_registry).toLowerCase() !== config.destinationRegistry.toLowerCase()) {
        throw new Error('Deployed adapter configuration does not match this agreement and frontend contract configuration.');
      }

      const sourceProvider = new JsonRpcProvider(CHAINS.base.rpc);
      const receipt = await sourceProvider.getTransactionReceipt(a.sourceTx);
      if (!receipt || receipt.status !== 1) throw new Error('The source payment receipt is missing or reverted.');
      const sourceInterface = new Interface(SOURCE_ABI);
      const log = receipt.logs.find(item => {
        if (item.address.toLowerCase() !== config.sourceEmitter.toLowerCase() || item.topics[0]?.toLowerCase() !== SOURCE_SCHEMA.toLowerCase()) return false;
        try {
          const parsed = sourceInterface.parseLog(item);
          return parsed.args.agreementId.toLowerCase() === a.id.toLowerCase()
            && parsed.args.nonce.toLowerCase() === a.nonce.toLowerCase();
        } catch { return false; }
      });
      if (!log) throw new Error('Could not find the exact SourcePayment log for this agreement and nonce.');
      const parsed = sourceInterface.parseLog(log);
      const amount = BigInt(parsed.args.amount);
      if (amount < BigInt(adapterConfig.min_amount) || amount > BigInt(adapterConfig.max_amount)) throw new Error('Source event amount is outside the adapter agreement bounds.');
      if (BigInt(adapterConfig.agreement_deadline) !== BigInt(a.deadline)) throw new Error('Adapter deadline does not match the registered agreement.');
      const candidate = {
        transaction_hash: fixedBytes(receipt.hash,32),
        block_hash: fixedBytes(receipt.blockHash,32),
        block_number: Number(receipt.blockNumber),
        transaction_index: Number(receipt.index),
        log_index: Number(log.index),
        emitter: fixedBytes(log.address,20),
        removed: false,
        data: log.data,
        topics: log.topics.map(topic=>fixedBytes(topic,32))
      };

      const wallet = await nearConnector.current.wallet();
      notify('info','Submitting source log to NEAR MPC','Approve the verification request in your NEAR wallet. The adapter requests finalized ForeignTx verification, followed by a separate EVM signature.');
      const submitted = await wallet.signAndSendTransaction({
        receiverId: adapter,
        actions: [{
          type:'FunctionCall',
          params:{
            methodName:'verify_base_log',
            args:candidate,
            gas:'100000000000000',
            deposit:'0'
          }
        }]
      });
      const nearTxHash = extractNearTxHash(submitted);
      if (!nearTxHash) throw new Error('The wallet did not return a NEAR transaction hash. If it redirected for approval, return to this page after wallet approval and retry.');
      notify('info','NEAR MPC verification running','Transaction '+nearTxHash.slice(0,12)+'… submitted. Waiting for ForeignTx and Chain Signatures callbacks.');
      const outcome = await waitForNearOutcome(nearTxHash,nearAccount);
      const signature = extractEvmSignature(outcome);
      if (!/^0x[0-9a-fA-F]{130}$/.test(signature)) throw new Error('Adapter returned an invalid EVM signature encoding.');

      const finalityPolicyId = asHex(adapterConfig.finality_policy_id);
      const fact = [
        CHAINS.base.id,
        log.address,
        SOURCE_SCHEMA,
        parsed.args.agreementId,
        receipt.blockHash,
        receipt.hash,
        BigInt(receipt.blockNumber),
        BigInt(log.index),
        parsed.args.asset,
        amount,
        parsed.args.recipient,
        BigInt(parsed.args.sourceTimestamp),
        BigInt(adapterConfig.agreement_deadline),
        Number(adapterConfig.signer_version),
        Number(parsed.args.agreementVersion),
        finalityPolicyId,
        parsed.args.nonce
      ];
      await switchTo('destination');
      const evmProvider = new BrowserProvider(window.ethereum);
      const signer = await evmProvider.getSigner();
      const registry = new (await import('ethers')).Contract(config.destinationRegistry,REGISTRY_ABI,signer);
      const verifierAddress = await registry.verifier();
      const configuredVerifier = asHex(adapterConfig.verifier_contract);
      if (verifierAddress.toLowerCase() !== configuredVerifier.toLowerCase()) {
        throw new Error('Destination verifier does not match the verifier bound into the NEAR MPC signature.');
      }
      notify('info','Submitting accepted fact','MPC signature received. Submit the signed fact to the Ethereum Sepolia registry.');
      const consumeTx = await registry.consumeSourceFact(fact,signature);
      const consumeReceipt = await consumeTx.wait();
      saveAgreements(agreements.map(x => x.id===a.id ? {...x,status:'verified',nearTx:nearTxHash,verifyTx:consumeReceipt.hash} : x));
      addEvent({type:'verification',title:'NEAR MPC fact accepted',detail:'Finalized Base log authenticated and accepted by Ethereum registry · '+compact(a.id),hash:consumeReceipt.hash,chain:'destination'});
      notify('success','Source fact accepted','NEAR MPC returned a valid signature and the destination registry accepted the exact source fact.');
    } catch(e) {
      notify('error','NEAR verification failed',e.shortMessage || e.reason || e.message || 'The verification flow did not complete.');
    } finally { setBusy(''); }
  }

  async function checkFact(a) {
    setBusy('check');
    try {
      const p = new JsonRpcProvider(CHAINS.destination.rpc);
      const registry = new (await import('ethers')).Contract(config.destinationRegistry,REGISTRY_ABI,p);
      const result = await registry.getAcceptedFact(a.id,a.nonce);
      if (!result.exists) return notify('info','Awaiting MPC verification','No accepted fact is recorded in the destination registry yet. The NEAR adapter must submit it first.');
      const f = result.fact;
      if (f.nonce.toLowerCase()!==a.nonce.toLowerCase() || f.sourceTxHash.toLowerCase()!==a.sourceTx.toLowerCase() || f.emitter.toLowerCase()!==config.sourceEmitter.toLowerCase() || f.asset.toLowerCase()!==config.sourceToken.toLowerCase() || f.recipient.toLowerCase()!==a.recipient.toLowerCase()) throw new Error('Accepted fact does not match the recorded payment and agreement.');
      saveAgreements(agreements.map(x => x.id===a.id ? {...x,status:'verified'} : x));
      addEvent({type:'verification',title:'Source fact accepted',detail:'Registry accepted matching MPC attestation · '+compact(a.id),chain:'destination'});
      notify('success','Source fact accepted','The registry exposes the matching accepted fact. Destination settlement can now be attempted.');
    } catch(e) { notify('error','Verification check failed',e.shortMessage || e.message); }
    finally { setBusy(''); }
  }

  async function settle(a) {
    if (!account) return notify('warning','Connect wallet','A wallet must submit the destination settlement transaction.');
    setBusy('settle');
    try {
      await switchTo('destination');
      const readProvider = new JsonRpcProvider(CHAINS.destination.rpc);
      const registry = new (await import('ethers')).Contract(config.destinationRegistry,REGISTRY_ABI,readProvider);
      const configured = await registry.settlementAdapter();
      if (!validAddress(configured) || configured.toLowerCase()!==config.destinationSettlement.toLowerCase()) throw new Error('Settlement address does not match registry.settlementAdapter().');
      const fact = await registry.getAcceptedFact(a.id,a.nonce);
      if (!fact.exists) throw new Error('Registry has not accepted this source fact.');
      const p = new BrowserProvider(window.ethereum), signer = await p.getSigner();
      const settlement = new (await import('ethers')).Contract(config.destinationSettlement,SETTLEMENT_ABI,signer);
      const tx = await settlement.settleERC20(a.id,a.nonce);
      notify('info','Enforcing destination outcome','Waiting for Ethereum Sepolia confirmation.');
      const receipt = await tx.wait();
      saveAgreements(agreements.map(x => x.id===a.id ? {...x,status:'settled',settlementTx:receipt.hash} : x));
      addEvent({type:'settlement',title:'Destination outcome enforced',detail:compact(a.id),hash:receipt.hash,chain:'destination'});
      notify('success','Outcome enforced','The destination settlement transaction was confirmed.');
    } catch(e) { notify('error','Settlement failed',e.shortMessage || e.reason || e.message); }
    finally { setBusy(''); }
  }

  async function mintDemo() {
    if (!account || !validAddress(config.sourceToken)) return notify('warning','Token unavailable','Connect wallet and configure the source token.');
    setBusy('mint');
    try {
      await switchTo('base');
      const p = new BrowserProvider(window.ethereum), signer = await p.getSigner();
      const token = new (await import('ethers')).Contract(config.sourceToken,TOKEN_ABI,signer);
      if ((await token.owner()).toLowerCase()!==account.toLowerCase()) throw new Error('Only the demo token owner can mint.');
      const tx = await token.mint(account,parseUnits('1000',Number(await token.decimals())));
      await tx.wait(); addEvent({type:'mint',title:'Demo tokens minted',detail:'1,000 tokens on Base Sepolia',hash:tx.hash,chain:'base'});
      notify('success','Tokens minted','1,000 demo tokens added to your wallet.');
    } catch(e) { notify('error','Mint failed',e.shortMessage || e.reason || e.message); }
    finally { setBusy(''); }
  }

  async function fundSettlement() {
    if (!account || !validAddress(config.destinationToken) || !validAddress(config.destinationSettlement)) return notify('warning','Configuration missing','Add destination token and settlement addresses.');
    const value = window.prompt('Amount to pre-fund settlement','100');
    if (value===null || Number(value)<=0) return;
    setBusy('fund');
    try {
      await switchTo('destination');
      const p = new BrowserProvider(window.ethereum), signer = await p.getSigner();
      const token = new (await import('ethers')).Contract(config.destinationToken,TOKEN_ABI,signer);
      const tx = await token.transfer(config.destinationSettlement,parseUnits(value,Number(await token.decimals())));
      await tx.wait(); addEvent({type:'fund',title:'Settlement adapter funded',detail:value+' destination tokens',hash:tx.hash,chain:'destination'});
      notify('success','Settlement funded','Destination adapter received '+value+' tokens.');
    } catch(e) { notify('error','Funding failed',e.shortMessage || e.reason || e.message); }
    finally { setBusy(''); }
  }

  function saveConfig(next) { setConfig(next); localStorage.setItem('afterstate:config',JSON.stringify(next)); notify('success','Configuration saved','Saved locally in this browser.'); }
  const title = {overview:'Overview',agreements:'Agreements',activity:'Activity','setup':'Network setup'}[page];

  return <div className="shell">
    <aside className={'sidebar '+(mobile?'open':'')}>
      <div className="brand"><div className="mark"><i/><i/><i/></div><div><b>afterstate</b><small>EXECUTION ASSURANCE</small></div><button className="close-mobile" onClick={()=>setMobile(false)}><X size={17}/></button></div>
      <label className="nav-label">WORKSPACE</label>
      <nav>{[['overview',Gauge,'Overview'],['agreements',FileCheck2,'Agreements'],['activity',Activity,'Activity'],['setup',Settings2,'Network setup']].map(([id,Icon,label])=><button key={id} className={'nav-link '+(page===id?'selected':'')} onClick={()=>{setPage(id);setMobile(false);}}><Icon size={17}/>{label}{id==='setup' && setupCount<5 && <small>{setupCount}/5</small>}</button>)}</nav>
      <div className="sidebar-bottom"><div className="side-network"><span/> <div><b>Testnet environment</b><small>Base ↔ Ethereum Sepolia</small></div></div><small className="side-disclaimer"><Shield size={13}/> Experimental · unaudited</small></div>
    </aside>
    <main>
      <header className="topbar"><div className="crumb"><button className="menu" onClick={()=>setMobile(true)}><Menu size={18}/></button><span>Workspace</span><i>/</i><b>{title}</b></div><div className="top-actions"><span className="testnet"><i/> TESTNET</span><button className={'wallet '+(nearAccount?'connected':'')} onClick={connectNear} title={nearAccount || 'Connect NEAR testnet wallet'}>{nearAccount?'NEAR '+nearAccount.slice(0,8)+'…':'Connect NEAR'}</button>{account?<button className="wallet connected" onClick={()=>notify('info','Wallet connected',account+' · chain '+chainId)}><span/>{compact(account,5)} <ChevronDown size={13}/></button>:<button className="wallet" onClick={connect} disabled={busy==='connect'}><Wallet size={14}/>{busy==='connect'?'Connecting…':'Connect wallet'}</button>}</div></header>
      <div className="content">
        {page==='overview' && <>
          <div className="heading"><div><label className="eyebrow">CROSS-CHAIN EXECUTION LAYER</label><h1>Make outcomes <em>verifiable.</em></h1><p>Bind a source-chain event to a destination action—with cryptographic verification and explicit postconditions.</p></div><button className="primary" onClick={()=>setPage('agreements')}><Plus size={15}/> New agreement</button></div>
          <section className="route"><div className="route-heading"><div><label className="eyebrow">REFERENCE EXECUTION PATH</label><b>One fact. Two chains. An enforceable outcome.</b></div><span><RadioTower size={13}/> MPC-verified path</span></div><div className="route-flow"><Chain mark="B" name="Base Sepolia" detail="Source event" kind="base"/><Connector label="ForeignTx"/><Chain mark="N" name="NEAR MPC" detail="Verify + attest" kind="near"/><Connector label="EIP-712"/><Chain mark="Ξ" name="Ethereum Sepolia" detail="Enforce outcome" kind="eth"/></div><div className="route-foot"><span><LockKeyholeIcon/> Finalized source event</span><span><Fingerprint size={13}/> Domain-separated signer</span><span><BadgeCheck size={13}/> Exact balance postcondition</span></div></section>
          <div className="metrics"><Metric label="Agreements" value={agreements.length} note="Tracked in this browser"/><Metric label="Source payments" value={agreements.filter(a=>a.sourceTx).length} note="Confirmed on Base Sepolia"/><Metric label="Verified facts" value={agreements.filter(a=>['verified','settled'].includes(a.status)).length} note="Accepted by destination registry"/><Metric label="Outcomes enforced" value={agreements.filter(a=>a.status==='settled').length} note="Settlement confirmed"/></div>
          <div className="columns"><section className="panel"><PanelTitle title="Recent execution activity" sub="Transactions initiated through this interface." action="View all" onAction={()=>setPage('activity')}/>{events.length?<div className="event-list">{events.slice(0,5).map(e=><EventRow key={e.id} event={e}/>)}</div>:<Empty icon={Activity} title="No activity yet" text="Register an agreement and submit a source payment to begin tracking execution." action="Create agreement" onClick={()=>setPage('agreements')}/>}</section><section className="panel"><PanelTitle title="Protocol readiness" sub="Contract configuration in this browser."/><div className="readiness"><b>{setupCount}<small>/5</small></b><div><strong>{setupCount===5?'Addresses configured':'Deployment setup'}</strong><span>Contract addresses supplied</span></div></div><div className="checks">{[['Source token',config.sourceToken],['Source emitter',config.sourceEmitter],['Destination token',config.destinationToken],['Destination registry',config.destinationRegistry],['Settlement adapter',config.destinationSettlement]].map(([l,v])=><div key={l}><span className={validAddress(v)?'checked':''}>{validAddress(v)?<Check size={10}/>:''}</span>{l}<small>{validAddress(v)?'Added':'Missing'}</small></div>)}</div><button className="secondary full" onClick={()=>setPage('setup')}>Configure contracts <ArrowRight size={13}/></button></section></div>
          <Notice/>
        </>}
        {page==='agreements' && <><div className="heading"><div><label className="eyebrow">AGREEMENT MANAGEMENT</label><h1>Execution <em>agreements.</em></h1><p>Define the source event constraints before a payment is made.</p></div><button className="secondary" onClick={mintDemo} disabled={busy==='mint'}><Zap size={14}/>{busy==='mint'?'Minting…':'Mint demo tokens'}</button></div><div className="agreement-layout"><section className="panel create"><PanelTitle title="Create an agreement" sub="Registered on Ethereum Sepolia. Registry owner authorization required."/><form onSubmit={createAgreement}><Field label="Destination recipient" value={form.recipient} onChange={v=>setForm({...form,recipient:v})} placeholder="0x…"/><div className="field-row"><Field label="Minimum amount" value={form.min} onChange={v=>setForm({...form,min:v})} type="number"/><Field label="Maximum amount" value={form.max} onChange={v=>setForm({...form,max:v})} type="number"/></div><Field label="Agreement deadline" value={form.deadline} onChange={v=>setForm({...form,deadline:v})} type="datetime-local"/><div className="form-summary"><span>Source · Base Sepolia (84532)</span><span>Destination · Ethereum Sepolia (11155111)</span><span>Event schema · SourcePayment v1</span></div><button className="primary full" type="submit" disabled={busy==='create'||!account||!ready}>{busy==='create'?'Registering…':<><Plus size={15}/> Register agreement</>}</button>{!account&&<small className="hint">Connect the registry-owner wallet first.</small>}{!ready&&<small className="hint">Finish Network setup. v0.1 requires the same ERC-20 address on both EVM chains.</small>}</form></section><section className="panel"><PanelTitle title={'Your agreements · '+agreements.length} sub="Progress is shown as separate protocol milestones."/>{agreements.length?<div className="agreement-list">{agreements.map(a=><Agreement key={a.id} a={a} busy={busy} pay={()=>paySource(a)} check={()=>verifyWithNear(a)} settle={()=>settle(a)}/>)}</div>:<Empty icon={Layers3} title="No agreements registered" text="Create an agreement to define the exact source event and destination constraints."/ >}</section></div><Notice/></>}
        {page==='activity' && <><div className="heading"><div><label className="eyebrow">AUDIT TRAIL</label><h1>Execution <em>activity.</em></h1><p>Local transaction index. Block explorer links are the transaction source of truth.</p></div><button className="secondary" onClick={()=>saveEvents([])} disabled={!events.length}>Clear local history</button></div><section className="panel"><PanelTitle title="Transaction history" sub="This history is stored locally in this browser."/><div className="event-list">{events.length?events.map(e=><EventRow key={e.id} event={e}/>):<Empty icon={Activity} title="Nothing recorded yet" text="Agreement, payment, and settlement activity will appear here."/>}</div></section></>}
        {page==='setup' && <><div className="heading"><div><label className="eyebrow">DEPLOYMENT CONFIGURATION</label><h1>Network <em>setup.</em></h1><p>Connect the interface to deployed reference contracts. Values are stored in this browser only.</p></div><button className="secondary" onClick={refreshNetwork} disabled={network.loading}><RefreshCw size={14}/> Refresh on-chain state</button></div><div className="setup-grid"><section className="panel"><PanelTitle title="Contract addresses" sub="Paste addresses after the EVM deployment is complete."/><Settings config={config} save={saveConfig}/></section><section className="panel"><PanelTitle title="Destination registry state" sub="Read-only values from Ethereum Sepolia."/><div className="live-label"><i/> {network.loading?'CHECKING':'RPC READ'}</div>{network.error?<p className="error-text"><CircleAlert size={15}/>{network.error}</p>:<div className="state">{[['Registry owner',network.owner],['Verifier',network.verifier],['Settlement adapter',network.adapter],['Token',network.symbol+' · '+network.decimals+' decimals']].map(([k,v])=><State key={k} label={k} value={v}/>)}</div>}<div className="adapter"><RadioTower size={17}/><div><b>NEAR testnet adapter account</b><code>{config.nearAdapterAccount}</code><small>Public account ID. Adapter deployment must be verified separately.</small></div></div><button className="secondary full" onClick={fundSettlement} disabled={!validAddress(config.destinationSettlement)||busy==='fund'}><ArrowDownLeft size={14}/>{busy==='fund'?'Funding…':'Pre-fund destination settlement'}</button></section></div><Notice/></>}
      </div>
      <footer><span>AFTERSTATE · PROTOCOL INTERFACE V0.1</span><span><a href="https://github.com/Godwin-web3/AfterState" target="_blank" rel="noreferrer">Source code <ExternalLink size={11}/></a> · Testnet only · unaudited</span></footer>
    </main>
    {toast&&<div className={'toast '+toast.kind}><div>{toast.kind==='success'?<CheckCircle2 size={18}/>:toast.kind==='warning'?<CircleAlert size={18}/>:toast.kind==='error'?<X size={18}/>:<Activity size={18}/>}</div><section><b>{toast.title}</b><p>{toast.message}</p></section><button onClick={()=>setToast(null)}><X size={14}/></button></div>}
  </div>;
}

function validNearAccountId(value){return typeof value==='string' && (/^[a-f0-9]{64}$/i.test(value) || /^[a-z0-9_-]+(\\.[a-z0-9_-]+)*$/.test(value));}\nfunction LockKeyholeIcon(){return <Shield size={13}/>}
function Chain({mark,name,detail,kind}){return <div className="chain"><i className={kind}>{mark}</i><div><b>{name}</b><small>{detail}</small></div></div>}
function Connector({label}){return <div className="connector"><i/><small>{label}</small></div>}
function Metric({label,value,note}){return <div className="metric"><span>{label}</span><b>{value}</b><small>{note}</small></div>}
function PanelTitle({title,sub,action,onAction}){return <div className="panel-title"><div><h2>{title}</h2><p>{sub}</p></div>{action&&<button className="link-button" onClick={onAction}>{action} <ArrowRight size={13}/></button>}</div>}
function Empty({icon:Icon,title,text,action,onClick}){return <div className="empty"><Icon size={22}/><b>{title}</b><p>{text}</p>{action&&<button className="link-button" onClick={onClick}>{action} <ArrowRight size={13}/></button>}</div>}
function Notice(){return <div className="protocol-notice"><Shield size={17}/><div><b>Understand the trust boundary.</b><p>NEAR MPC authenticates a foreign transaction observation under its provider and threshold assumptions; it is not a trustless EVM light client. Source payments are irreversible. Destination failure does not automatically refund them. This prototype is unaudited.</p></div><a href="https://github.com/Godwin-web3/AfterState/blob/main/docs/SECURITY_REVIEW.md" target="_blank" rel="noreferrer">Security model <ExternalLink size={11}/></a></div>}
function EventRow({event}){const chain=event.chain==='base'?CHAINS.base:CHAINS.destination;return <div className="event-row"><i>{event.type==='payment'?<ArrowUpRight size={15}/>:event.type==='settlement'?<CheckCircle2 size={15}/>:event.type==='agreement'?<FileCheck2 size={15}/>:<Zap size={15}/>}</i><div><b>{event.title}</b><small>{event.detail}</small></div><time>{new Date(event.at).toLocaleString()}</time>{event.hash&&<a href={chain.explorer+'/tx/'+event.hash} target="_blank" rel="noreferrer"><ExternalLink size={13}/></a>}</div>}
function Field({label,value,onChange,type='text',placeholder=''}){return <label className="field"><span>{label}</span><input type={type} value={value} onChange={e=>onChange(e.target.value)} placeholder={placeholder} spellCheck="false"/></label>}
function Settings({config,save}){const [draft,setDraft]=useState(config);useEffect(()=>setDraft(config),[config]);const fields=[['sourceToken','Base Sepolia · demo token'],['sourceEmitter','Base Sepolia · source emitter'],['destinationToken','Ethereum Sepolia · token (same address required)'],['destinationRegistry','Ethereum Sepolia · registry'],['destinationSettlement','Ethereum Sepolia · settlement adapter']];return <form className="settings" onSubmit={e=>{e.preventDefault();save(draft);}}>{fields.map(([k,l])=><Field key={k} label={l} value={draft[k]} onChange={v=>setDraft({...draft,[k]:v.trim()})} placeholder="0x…"/>)}<Field label="NEAR testnet adapter account" value={draft.nearAdapterAccount} onChange={v=>setDraft({...draft,nearAdapterAccount:v.trim()})}/><small>Never enter a seed phrase, private key, or signing password here.</small><button className="primary" type="submit"><Check size={14}/> Save configuration</button></form>}
function State({label,value}){return <div className="state-row"><span>{label}</span><code>{value||'—'}</code>{value&&value.startsWith('0x')&&<button onClick={()=>navigator.clipboard?.writeText(value)}><Copy size={12}/></button>}</div>}
function Agreement({a,busy,pay,check,settle}){const steps=['Agreement','Source event','Verified','Outcome'];const n=a.status==='settled'?4:a.status==='verified'?3:a.status==='source-paid'?2:1;return <article className="agreement"><div className="agreement-top"><div><b>{compact(a.id,8)}</b><small>Recipient {compact(a.recipient,5)}</small></div><span className={'pill '+a.status}>{a.status==='source-paid'?'Source paid':a.status==='verified'?'Fact verified':a.status==='settled'?'Outcome enforced':'Agreement active'}</span></div><div className="steps">{steps.map((s,i)=><div className={i<n?'done':''} key={s}><i/>{s}</div>)}</div><div className="agreement-bottom"><div><b>{a.min}–{a.max} {a.symbol||'ASD'}</b><small>Deadline {new Date(a.deadline*1000).toLocaleDateString()}</small></div><div className="actions">{a.status==='registered'&&<button onClick={pay} disabled={busy==='pay'}>{busy==='pay'?'Paying…':'Pay on Base'}</button>}{a.status==='source-paid'&&<button onClick={check} disabled={busy==='verify'}>{busy==='verify'?'Verifying…':'Verify with NEAR MPC'}</button>}{a.status==='verified'&&<button onClick={settle} disabled={busy==='settle'}>{busy==='settle'?'Settling…':'Enforce outcome'}</button>}</div></div>{a.sourceTx&&<small className="tx-link">Source transaction <a href={CHAINS.base.explorer+'/tx/'+a.sourceTx} target="_blank" rel="noreferrer">{compact(a.sourceTx,7)} <ExternalLink size={10}/></a></small>}</article>}
