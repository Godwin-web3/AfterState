import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Activity, ArrowDownLeft, ArrowLeftRight, ArrowRight, ArrowUpRight, BadgeCheck, Bell, Blocks,
  Check, CheckCircle2, ChevronDown, CircleAlert, CircleHelp, Clipboard, Clock3, Code2, Copy,
  ExternalLink, FileCheck2, Fingerprint, Gauge, Globe2, Layers3, Link2, LockKeyhole, Menu,
  Plus, RadioTower, RefreshCw, Rocket, Settings2, Shield, ShieldCheck, Wallet, X, Zap
} from 'lucide-react';
import { BrowserProvider, JsonRpcProvider, formatEther, parseUnits } from 'ethers';
import {
  CHAINS, DEFAULT_CONFIG, REGISTRY_ABI, SETTLEMENT_ABI, SOURCE_EMITTER_ABI, SOURCE_SCHEMA,
  TOKEN_ABI, compact, explorerTx, getContract, loadConfig, newBytes32, saveConfig, validAddress
} from './protocol';

const NAV = [
  { id: 'overview', label: 'Overview', icon: Gauge },
  { id: 'agreements', label: 'Agreements', icon: FileCheck2 },
  { id: 'activity', label: 'Activity', icon: Activity },
  { id: 'setup', label: 'Network setup', icon: Settings2 }
];
const emptyForm = { recipient: '', minAmount: '1', maxAmount: '100', deadline: '' };

function App() {
  const [page, setPage] = useState('overview');
  const [config, setConfig] = useState(loadConfig);
  const [account, setAccount] = useState('');
  const [chainId, setChainId] = useState(null);
  const [connecting, setConnecting] = useState(false);
  const [busy, setBusy] = useState('');
  const [notice, setNotice] = useState(null);
  const [form, setForm] = useState(emptyForm);
  const [agreements, setAgreements] = useState(() => readLocal('afterstate:agreements', []));
  const [activity, setActivity] = useState(() => readLocal('afterstate:activity', []));
  const [networkState, setNetworkState] = useState({ checking: false, registryOwner: '', verifier: '', adapter: '', tokenSymbol: 'ASD', tokenDecimals: 6, error: '' });
  const [selectedAgreement, setSelectedAgreement] = useState(null);
  const [mobileNav, setMobileNav] = useState(false);

  const isConnected ...[truncated]