# Cyberskills -> Rust mechanization catalog (817 skills)

## T1 (145)

| skill | rule_id | input | predicate | conv |
|---|---|---|---|---|
| implementing-just-in-time-access-provisioning | access.jit.duration-enforcement | json-config | Audit JIT requests for duration limits, missing approvers, and expired access not revoked | easy |
| analyzing-active-directory-acl-abuse | ad.acl.dangerous-permissions-on-sensitive-objects | ldap-directory-query | Object has ACE granting GenericAll/WriteDACL/WriteOwner/GenericWrite to non-admin principal | medium |
| performing-active-directory-compromise-investigation | ad.event-log.dcsync-detection | json-config | Parse Windows Security event logs (JSON) for DCSync (4662), Kerberoasting (4769), Golden Ticket (4768), and lateral movement indicators | easy |
| securing-agentic-ai-tool-invocation | ai.agent.tool-invocation-policy-enforcement | json-config | Validate tool call arguments against JSON schema allowlist; deny-by-default for unknown tools; require approval for high-impact tools (send_email, transfer_funds, run_shell) | easy |
| implementing-api-threat-protection-with-apigee | api.apigee.threat-policy-check | json-config | Verify Apigee proxy has JSONThreatProtection, XMLThreatProtection, RegularExpressionProtection policies | medium |
| exploiting-broken-function-level-authorization | api.bfla.admin-endpoint-access | http-request | Test API endpoints with regular user credentials to detect missing function-level authorization checks | medium |
| exploiting-idor-vulnerabilities | api.idor.predictable-id-enumeration | http-request | Detect sequential or predictable resource IDs in URL/query/body that bypass object-level authorization checks | easy |
| exploiting-mass-assignment-in-rest-apis | api.mass-assignment.unexpected-field-binding | json-request | Detect ORM autobinding of unexpected fields (role, isAdmin, verified, price) that bypass authorization | easy |
| implementing-api-security-testing-with-42crunch | api.owasp-api.security-audit | json-config | Audit OpenAPI spec for OWASP API 2023 risks (broken auth, broken object auth, mass assignment) | easy |
| implementing-api-rate-limiting-and-throttling | api.rate-limit.config-check | json-config | Validate rate limiting algorithms (token bucket, sliding window) are properly configured | medium |
| implementing-api-schema-validation-security | api.schema.validation-missing | json-config | Check OpenAPI/JSON schema for missing request/response validation and additionalProperties disabled | easy |
| detecting-shadow-api-endpoints | api.undocumented-endpoints | log-line | Parse access logs, normalize API paths, identify endpoints absent from OpenAPI spec | medium |
| implementing-application-whitelisting-with-applocker | app.applocker.enforcement-and-paths | json-config | Audit AppLocker policy XML for enforcement mode enabled and deny rules on risky paths (Temp/Downloads/AppData) | medium |
| performing-second-order-sql-injection | app.sql.second-order-injection-payload | json-config | Database dump contains SQL injection pattern (UNION SELECT / DROP TABLE / comment syntax) in stored fields OR source code has unsafe string concatenation in SQL query construction | easy |
| performing-jwt-none-algorithm-attack | auth.jwt.alg-none-signature-bypass | jwt-token | Detect JWT tokens with alg field set to none or empty value via base64 decode | easy |
| exploiting-jwt-algorithm-confusion-attack | auth.jwt.algorithm-confusion | jwt-token | Detect JWT tokens with weak algorithms (none, HS256 when RS256 expected) or missing algorithm enforcement | easy |
| implementing-jwt-signing-and-verification | auth.jwt.signing-verification | source-code | Verify JWT token structure, signature validity, algorithm strength, and expiry using crypto libs | medium |
| detecting-golden-ticket-forgery | auth.kerberos.golden-ticket-forgery | windows-event-log | Detects RC4 (0x17) encryption in TGS (Event 4769) when AES enforced; orphaned TGS without preceding TGT (Event 4768); abnormal ticket lifetime gaps exceeding MaxTicketAge | medium |
| detecting-kerberoasting-attacks | auth.kerberos.kerberoasting-detection | windows-evtx-4769 | Detects RC4-HMAC (0x17/0x18) encryption in TGS-REQ; flags high volume TGS requests from single source targeting multiple SPNs; detects multiple logons from same IP | medium |
| detecting-pass-the-ticket-attacks | auth.kerberos.ptt-attack-detection | windows-event-log-4768-4769-4771 | Parses Events 4768/4769/4771 for RC4 downgrade; detects orphaned TGS (4769 without 4768); identifies requests with non-standard encryption; flags krbtgt service anomalies | medium |
| conducting-pass-the-ticket-attack | auth.kerberos.rc4-ticket-request | json-log | Detects RC4-encrypted Kerberos TGT/service ticket requests (EventID 4768/4769 with TicketEncryptionType=0x17) | easy |
| detecting-pass-the-hash-attacks | auth.ntlm.pth-detection | windows-evtx-4624 | Filters Event 4624 for Logon_Type=3 (Network) with AuthenticationPackage=NTLM; detects multiple targets from single source; correlates with credential dumping indicators | medium |
| detecting-ntlm-relay-with-event-correlation | auth.ntlm.relay-attack-correlation | windows-evtx-4624-4776 | Correlates Event 4624 across hosts for same IP; detects IP-to-hostname mismatches; monitors SMB/LDAP signing enforcement; flags NTLM downgrade (NTLMv2 to NTLMv1) | hard |
| exploiting-oauth-misconfiguration | auth.oauth.redirect-uri-bypass | http-request | Detect weak redirect_uri validation and token leakage in OAuth authorization flows | medium |
| detecting-rdp-brute-force-attacks | auth.rdp-brute-force | windows-evtx | Count RDP failed logons (4625) per source IP, flag if >= threshold with subsequent success | medium |
| implementing-saml-sso-with-okta | auth.saml-configuration-weakness | tls-config | SAML metadata uses SHA-1 signatures (not SHA-256), lacks AudienceRestriction, missing assertion encryption, or expired certs | medium |
| implementing-identity-verification-for-zero-trust | auth.zero-trust.mfa-phishing-resistant | json-config | Assess auth methods against zero-trust baseline (MFA enabled, phishing-resistant, FIDO2, passwordless) | easy |
| implementing-zero-knowledge-proof-for-authentication | auth.zkp.schnorr-proof-validation | source-code | Validate Schnorr ZKP proof: verify g^s ≡ t*y^c mod p; reject proof if verification fails | easy |
| validating-backup-integrity-for-recovery | backup.integrity-validation | directory | Verify backup files match baseline hashes, entropy <7.9, no ransomware extensions or notes | easy |
| performing-thick-client-application-penetration-test | binary.hardcoded-credential-references | pe-executable | Binary strings contain credential-related keywords (password, token, api_key, connectionstring) or SQL query patterns | easy |
| performing-static-malware-analysis-with-pe-studio | binary.pe-suspicious-imports | pe-executable | PE file contains suspicious import functions (VirtualAllocEx, CreateRemoteThread, SetWindowsHookEx, RegSetValueEx) mapped to process injection or keylogging | medium |
| securing-github-actions-workflows | cicd.github-actions.workflow-hardening | source-code | Parse YAML workflow files; detect unpinned actions (ref not matching 40-char SHA); flag overly permissive permissions; detect script injection in expressions | easy |
| detecting-supply-chain-attacks-in-ci-cd | cicd.workflow-permission-check | yaml-config | Check GitHub Actions workflows for overpermissive permissions and unpinned action versions | easy |
| securing-aws-iam-permissions | cloud.aws.iam-least-privilege-validation | cloud-json | Parse IAM policy; detect wildcard actions (*) on Resource '*'; flag roles with AdministratorAccess/PowerUserAccess; find access keys >90 days old | medium |
| securing-aws-lambda-execution-roles | cloud.aws.lambda-role-least-privilege | cloud-json | Audit Lambda execution role policies; flag broad managed policies (S3FullAccess, DynamoDBFullAccess); detect wildcard actions on all resources | medium |
| remediating-s3-bucket-misconfiguration | cloud.aws.s3-public-access-block-required | cloud-json | Check S3 bucket PublicAccessBlockConfiguration; require all four flags (BlockPublicAcls, IgnorePublicAcls, BlockPublicPolicy, RestrictPublicBuckets) = true | easy |
| securing-api-gateway-with-aws-waf | cloud.aws.waf-rules-configured | cloud-json | Check WAF Web ACL has managed rule groups enabled (CommonRuleSet, KnownBadInputs); verify rate limiting and bot control rules present | medium |
| performing-serverless-function-security-review | cloud.lambda.overprivileged-role | cloud-json | Lambda execution role has AdministratorAccess policy OR IAM policy allows wildcard actions (*) OR environment variables contain secret/password/key patterns OR function allows public invocation (Principal: '*') | medium |
| implementing-cloud-waf-rules | cloud.waf.managed-rule-enforcement | cloud-json | Verify AWS WAF Web ACL has managed rule groups enabled with non-Count action | medium |
| exploiting-insecure-deserialization | code.serialization.unsafe-magic-bytes | source-code | Detect unsafe deserialization via magic byte signatures (Java 0xaced0005, .NET 0x2f774500, PHP serialization patterns) | medium |
| hardening-docker-containers-for-production | container.docker-container-hardening | json-config | Audit Docker container config against CIS Docker Benchmark controls | easy |
| hardening-docker-daemon-configuration | container.docker-daemon-hardening | json-config | Verify daemon.json settings against CIS Docker daemon hardening benchmarks | easy |
| escaping-containers-to-host | container.escape-vector-detection | none | Check /proc for privileged capabilities and dangerous mounts indicating container escape feasibility | hard |
| securing-container-registry-with-harbor | container.harbor.project-security-baseline | json-config | Check Harbor project metadata: require auto_scan=true, prevent_vul=true, enable_content_trust=true, enable_content_trust_cosign=true; flag if public=true | medium |
| performing-ssl-certificate-lifecycle-management | crypto.cert.expired-or-invalid | tls-config | Certificate not-after date is in past OR days-remaining < threshold (30 days) OR certificate self-signed OR signature algorithm deprecated (SHA1) OR key size inadequate (RSA < 2048) | easy |
| implementing-end-to-end-encryption-for-messaging | crypto.e2e.x25519-aes-gcm-encryption | none | Verify X25519 key exchange and AES-256-GCM encryption/decryption operations are deterministic | easy |
| implementing-rsa-key-pair-management | crypto.rsa-key-strength | tls-config | RSA key size is <2048 bits OR not encrypted with passphrase OR uses unsafe serialization format | easy |
| implementing-digital-signatures-with-ed25519 | crypto.signature.ed25519-verification | source-code | Verify Ed25519 signature validity over a file using public key | easy |
| migrating-to-post-quantum-cryptography | crypto.tls.pqc-algorithm-inventory | tls-config | Classify TLS cert public-key algorithm as quantum-vulnerable (RSA/EC/DSA/DH) or quantum-safe (ML-KEM/ML-DSA/SLH-DSA); flag vulnerable algorithms | easy |
| implementing-endpoint-dlp-controls | data.dlp.sensitive-data-pattern-detection | source-code | Detect SSN, credit card, AWS keys, private keys, API keys in file contents via regex | easy |
| detecting-mobile-malware-behavior | detect.android.dangerous-permissions | android-manifest | Static analysis of APK manifest for DANGEROUS_ANDROID_PERMISSIONS (SEND_SMS, READ_CALL_LOG, BIND_DEVICE_ADMIN); detects suspicious receivers and malware API patterns (Runtime.exec, DexClassLoader) | medium |
| analyzing-outlook-pst-for-email-forensics | detect.email.pst-artifact-extraction | pst-file | Parse PST/OST binary format to extract emails, metadata, attachments, and deleted items | medium |
| hunting-for-defense-evasion-via-timestomping | detect.file.timestamp-anomaly | cloud-json | NTFS MFT $STANDARD_INFORMATION and $FILE_NAME timestamp discrepancies indicating tampering | easy |
| detecting-process-hollowing-technique | detect.injection.process-hollowing | windows-sysmon-evtx | Detects suspicious parent-child process pairs (cmd->svchost, powershell->svchost); flags suspended process creation; monitors memory allocation in remote processes | medium |
| detecting-living-off-the-land-with-lolbas | detect.lolbas.arg-pattern-match | windows-process-telemetry | Matches process command-line arguments against LOLBin signature suspicious_args lists (certutil -urlcache, regsvr32 /i:http); checks MITRE mapping | easy |
| detecting-living-off-the-land-attacks | detect.lolbin.suspicious-execution | windows-sysmon-evtx | Regex pattern matching for LOLBin command-lines (certutil -urlcache, mshta URLs, rundll32 javascript); checks parent-child process relationships; monitors network connections from LOLBins | medium |
| performing-malware-persistence-investigation | detect.malware.suspicious-registry-value | windows-registry | Suspicious keywords in Run/RunOnce/Services registry values indicate persistence mechanism | easy |
| detecting-mimikatz-execution-patterns | detect.mimikatz.execution-signature | windows-sysmon-evtx | Matches command-line patterns (sekurlsa::logonpasswords, lsadump::dcsync, kerberos::golden) and binary indicators (mimikatz.exe, mimilib.dll); detects LSASS dump via comsvcs/procdump | medium |
| detecting-privilege-escalation-attempts | detect.privesc.exploit-pattern | windows-sysmon-evtx + linux-logs | Matches Windows patterns (UAC bypass tools, token manipulation, service path abuse, potato exploits); matches Linux patterns (sudo enumeration, SUID, CVE-specific exploit names) | medium |
| analyzing-lnk-file-and-jump-list-artifacts | detect.windows.lnk-jump-list-access | lnk-file | Parse Windows LNK/Jump List binary format to extract file access evidence and execution history | medium |
| analyzing-mft-for-deleted-file-recovery | detect.windows.mft-deleted-file-metadata | ntfs-mft-binary | Parse NTFS MFT records (1024-byte entries) to extract file metadata, timestamps, and state flags | medium |
| analyzing-prefetch-files-for-execution-history | detect.windows.prefetch-execution-evidence | prefetch-file | Parse Windows Prefetch binary format (versions 17-30) to extract execution timestamps and counts | medium |
| performing-dmarc-policy-enforcement-rollout | email-auth.dns.dmarc-policy-misconfiguration | json-config | Parse and validate DMARC DNS TXT records for proper policy progression (p=none -> p=quarantine -> p=reject) and alignment requirements | easy |
| implementing-dmarc-dkim-spf-email-security | email.dns.dkim-spf-dmarc-validation | json-config | Verify domain has valid DKIM, SPF, and DMARC DNS records configured | medium |
| conducting-phishing-incident-response | email.phishing.auth-failure-indicators | email-eml | Detects phishing emails with SPF/DKIM/DMARC authentication failures and credential harvesting URLs | medium |
| detecting-spearphishing-with-email-gateway | email.spoofing-detection | email-headers | Check SPF/DKIM/DMARC authentication, detect spoofing and urgency keywords | easy |
| analyzing-email-headers-for-phishing-investigation | email.spoofing.spf-dkim-dmarc-alignment | email-headers | Parse RFC 5322 headers and extract SPF/DKIM/DMARC results, From/Reply-To mismatch, URL obfuscation | easy |
| performing-endpoint-vulnerability-remediation | endpoint.patch.missing-updates | vulnerability-scan-csv | Parse vulnerability scan CSV and identify missing patches via wmic query | medium |
| detecting-ransomware-encryption-behavior | file.entropy-encryption-detection | file-system | Identify files with entropy >= 7.5 or ransomware-associated extensions or ransom note patterns | easy |
| detecting-stuxnet-style-attacks | file.stuxnet-ioc-match | file-system | Match file hashes and registry keys against known Stuxnet IOCs | easy |
| hunting-bootkits-in-efi-system-partition | firmware.bootkit-efi-detection | filesystem-efi | Detect malicious EFI binaries via hash baseline and signature anomalies | medium |
| analyzing-uefi-bootkit-persistence | firmware.bootkit-signature-match | none | Detect UEFI bootkit families via firmware region signatures | hard |
| extracting-browser-history-artifacts | forensic.browser-history-extraction | sqlite-db | Extract Chrome/Firefox/Edge browsing history from SQLite database files | easy |
| extracting-windows-event-logs-artifacts | forensic.evtx-event-extraction | evtx | Extract and classify critical Windows event IDs from EVTX files | easy |
| analyzing-windows-amcache-artifacts | forensics.amcache-execution-history | registry-hive | Parse Amcache.hve to extract program execution history | easy |
| performing-credential-access-with-lazagne | forensics.endpoint.credential-dumping-artifacts | log-line | Detect indicators of LaZagne tool execution via presence of specific file paths, process names, or Windows event log entries associated with credential harvesting | medium |
| analyzing-windows-lnk-files-for-artifacts | forensics.lnk-file-parser | none | Parse LNK binary format to extract target path and timestamps | medium |
| performing-mobile-device-forensics-with-cellebrite | forensics.mobile.extract-from-sqlite | sqlite-database | Query SQLite mmssms.db for SMS/MMS messages and extract contacts from Android device extraction | easy |
| analyzing-windows-prefetch-with-python | forensics.prefetch-execution-history | none | Parse Prefetch binary format to extract execution timestamps | medium |
| analyzing-windows-registry-for-artifacts | forensics.registry-autorun-extraction | registry-hive | Parse registry hives to extract autorun and UserAssist entries | easy |
| analyzing-windows-shellbag-artifacts | forensics.shellbag-folder-access | registry-hive | Parse ShellBag entries to reconstruct folder access history | medium |
| performing-sqlite-database-forensics | forensics.sqlite.deleted-record-recovery | json-config | SQLite database has freelist pages containing deleted records OR WAL file present indicating uncommitted transactions OR database page header mismatch from baseline | medium |
| analyzing-usb-device-connection-history | forensics.usb-device-enumeration | registry-hive | Parse SYSTEM hive USBSTOR keys to enumerate USB devices | easy |
| implementing-network-segmentation-with-firewall-zones | fw.zones.least-privilege | json-config | Firewall rules must not allow all services between zones; untrust->trust blocked; allow rules must log | easy |
| implementing-hipaa-security-rule-safeguards | hipaa.safeguard.gap-assessment | json-config | Score HIPAA safeguard implementation status (required vs addressable) and detect gaps | easy |
| performing-privileged-account-access-review | iam.account.stale-access | csv-config | Account not used within threshold days OR no recertification within interval OR shared account pattern with no owner | easy |
| configuring-active-directory-tiered-model | iam.activedir.tier-violation-detection | ldap-query | Audits AD tiering model: detects privileged accounts with password-never-expires or accounts spanning multiple tiers | medium |
| configuring-ldap-security-hardening | iam.ldap.misconfig-detection | ldap-service | Detects LDAP misconfigurations: anonymous binding allowed on port 389, LDAPS unavailable on 636, or unsigned/unbound channels | medium |
| implementing-ics-firewall-with-tofino | ics.firewall.allow-any-any-detection | json-config | Detect overly permissive ICS firewall rules (allow-any to any, risky Modbus functions) | easy |
| implementing-iec-62443-security-zones | ics.iec-62443.security-level-gap | json-config | Audit IEC 62443 zone architecture for SL gaps and undefined security level targets | medium |
| implementing-iso-27001-information-security-management | iso.27001.soa-completeness | json-config | Assess Statement of Applicability completeness and check for missing justifications on excluded controls | easy |
| exploiting-prototype-pollution-in-javascript | js.prototype.property-injection | source-code | Detect unsafe Object.assign, spread, or merge operations on untrusted JSON that enable __proto__ pollution | hard |
| analyzing-kubernetes-audit-logs | k8s.audit.rbac-and-secret-access-detection | k8s-audit-json | Detect pods/exec, secrets access, RBAC binding changes, privileged pod creation, anonymous access | easy |
| securing-helm-chart-deployments | k8s.helm.template-security-baseline | k8s-yaml | Scan rendered Helm templates for privileged=true, hostNetwork=true, hostPID=true, runAsUser=0, allowPrivilegeEscalation=true; require readOnlyRootFilesystem=true and runAsNonRoot=true | medium |
| implementing-network-policies-for-kubernetes | k8s.netpol.pod-coverage-required | k8s-yaml | All pods must be covered by at least one NetworkPolicy with ingress or egress rules | easy |
| implementing-kubernetes-network-policy-with-calico | k8s.netpol.policy-coverage | k8s-yaml | Audit Calico network policies for ingress/egress gaps, uncovered namespaces, and misaligned selectors | medium |
| performing-container-escape-detection | k8s.pod.dangerous-capabilities-and-privesc | k8s-yaml | Detect privileged containers or pods with dangerous Linux capabilities (SYS_ADMIN, SYS_PTRACE, NET_ADMIN) or host path mounts to sensitive paths | easy |
| implementing-pod-security-admission-controller | k8s.psa.baseline-enforced | k8s-yaml | All namespaces must have pod-security.kubernetes.io/enforce label set to baseline or restricted | easy |
| implementing-kubernetes-pod-security-standards | k8s.pss.baseline-restricted-violations | k8s-yaml | Check pod security standard labels and detect baseline/restricted violations (hostNetwork, privileged, ASLR) | medium |
| implementing-rbac-hardening-for-kubernetes | k8s.rbac.overprivileged-role | k8s-yaml | ClusterRole/Role contains wildcard verbs+resources, secret read access, pod exec, or RBAC escalation permissions | medium |
| performing-log-source-onboarding-in-siem | log.format.detect-type | log-line | Detect log format (syslog RFC3164/RFC5424/CEF/LEEF/JSON/CSV/Windows event) from sample lines | easy |
| implementing-log-integrity-with-blockchain | logging.integrity.hash-chain-verification | log-line | Verify log entry integrity using SHA-256 hash chain linking to detect tampering | easy |
| extracting-config-from-agent-tesla-rat | malware.agent-tesla-config-detection | binary | Detect Agent Tesla RAT configuration strings and C2 endpoint indicators | medium |
| analyzing-cobalt-strike-beacon-configuration | malware.cobalt-strike.beacon-config-extraction | pe-binary | Extract beacon TLV configuration from PE .data section via XOR decoding and field parsing | easy |
| extracting-iocs-from-malware-samples | malware.ioc-extraction | binary | Extract IOCs (IPs, domains, URLs, hashes, emails) from malware samples via regex | medium |
| analyzing-supply-chain-malware-artifacts | malware.pe-section-entropy-anomaly | source-code | Detect new/modified PE sections with high entropy in trojanized binary | medium |
| performing-yara-rule-development-for-detection | malware.yara-rule-generation | pe-executable | Extract high-scoring unique strings and byte patterns from PE files for YARA rule generation (length-based, CamelCase, path patterns) | medium |
| exploiting-deeplink-vulnerabilities | mobile.deeplink.unvalidated-handler | source-code | Extract Android intent-filter schemes and iOS URL scheme handlers to identify unvalidated deep link parameters | medium |
| detecting-lateral-movement-in-network | net.lateral.movement-correlation | zeek-logs + windows-evtx | Parses Zeek conn.log for SMB (445) / RDP (3389) / WinRM (5985-5986) internal connections; correlates with Windows auth logs (4624 LogonType 3/10) | hard |
| detecting-lateral-movement-with-zeek | net.lateral.smb-dce-rpc-abuse | zeek-logs | Parses Zeek conn/smb_mapping/smb_files/dce_rpc/ntlm/kerberos logs; detects admin share access; PsExec-style DCE-RPC service creation; Pass-the-Hash NTLM correlation | hard |
| performing-packet-injection-attack | net.packet.craft-send-test | none | Craft and send TCP/UDP/ICMP packets with custom flags, payloads, and options for security testing | easy |
| performing-network-packet-capture-analysis | net.pcap.extract-flows | pcap | Parse PCAP binary format to extract IP conversations, protocols, and port usage | medium |
| implementing-syslog-centralization-with-rsyslog | net.rsyslog-tls-encryption | json-config | Rsyslog server config missing TLS stream driver OR missing x509 cert auth OR incorrect cipher suite (<TLS 1.2) | easy |
| detecting-network-scanning-with-ids-signatures | net.scanning.port-scan-detection | ids-alert-logs + connection-logs | Parses Suricata EVE JSON; counts unique ports per source (>20 = SYN scan); detects nmap/masscan signatures; flags host sweeps (>10 unique hosts); Service enumeration detection | medium |
| performing-ot-network-security-assessment | ot.asset.purdue-level-validation | csv-config | Check asset inventory compliance with Purdue reference model zones and end-of-life status | easy |
| performing-oil-gas-cybersecurity-assessment | ot.asset.zone-mismatch-detection | csv-config | Verify OT assets match expected Purdue model zone per IEC62443 categories | easy |
| detecting-modbus-command-injection-attacks | ot.modbus.dangerous-write-functions | zeek-modbus-log | Parses Zeek Modbus log; flags dangerous function codes (5,6,15,16,22,23 - write operations); detects unauthorized masters; identifies malformed frames | medium |
| detecting-modbus-protocol-anomalies | ot.modbus.register-range-violation | zeek-modbus-log | Validates Modbus register addresses against VALID_REGISTER_RANGES; detects timing anomalies (unexpected intervals between requests); detects register read/write count violations | hard |
| implementing-patch-management-for-ot-systems | ot.patch.sla-compliance | json-config | Missing patches must be applied within severity-based SLA (critical 30d, high 60d); safety-critical devices require vendor validation | easy |
| detecting-malicious-scheduled-tasks-with-sysmon | persist.scheduled-task.suspicious-creation | windows-sysmon-evtx | Detects schtasks.exe creating tasks to SUSPICIOUS_PATHS (programdata, windows/temp, public); checks for encoded PowerShell, certutil, bitsadmin commands in task args | medium |
| hunting-for-startup-folder-persistence | persist.startup.suspicious-file | none | Executable files in %APPDATA%/Microsoft/Windows/Start Menu/Programs/Startup with suspicious extensions or creation timing | easy |
| analyzing-sbom-for-supply-chain-vulnerabilities | sbom.nvd-cve-correlation | json-config | Check SBOM components against NVD CVE database by CPE | easy |
| implementing-api-key-security-controls | secret.hardcoded-api-key | source-code | Find hardcoded API keys (AWS, Stripe, GitHub, OpenAI, etc.) via regex patterns in code | easy |
| performing-cryptographic-audit-of-application | source-code.crypto.weak-algorithm-usage | source-code | Detect weak cryptographic algorithms (MD5, SHA-1, RC4, DES) or insecure key sizes in source code via regex patterns | medium |
| triaging-vulnerabilities-with-ssvc-framework | ssvc.cve.prioritization | cve-metadata | Apply SSVC decision tree to exploitation-status, technical-impact, automatability, mission-prevalence; output Track/Track*/Attend/Act | medium |
| implementing-code-signing-for-artifacts | supply-chain.artifact.signature-verification | source-code | Verify artifact has valid Ed25519 or RSA signature matching declared public key | easy |
| hunting-for-supply-chain-compromise | supply-chain.package.compromised-dependency | package-manifest | Known compromised npm/PyPI packages or git-based dependencies in lockfiles or requirements | easy |
| detecting-dependency-confusion | supply-chain.package.dependency-confusion | package-manifest | Detect internal package names that exist on public registries (npm, PyPI, RubyGems) or are unclaimed and claimable | easy |
| implementing-sigstore-for-software-signing | supply-chain.sigstore-signature-validation | json-config | Container image/artifact lacks valid cosign signature OR Rekor transparency log entry missing/invalid OR identity binding expired | hard |
| processing-stix-taxii-feeds | threat-intel.stix.bundle-validation | json-config | Parse STIX 2.1 bundle; validate object types against schema; detect malformed/invalid STIX | medium |
| building-ioc-defanging-and-sharing-pipeline | threat.ioc.defang-format | json-config | IOC conforms to defanged format (dots->[], URLs use hxxp/hxxps) | easy |
| analyzing-threat-actor-ttps-with-mitre-attack | threat.mitre-attack-pattern-match | json-config | Match observed IOCs against MITRE ATT&CK technique database | easy |
| implementing-security-information-sharing-with-stix2 | threat.stix2-object-validation | json-config | STIX 2.1 object missing required fields (identity, created/modified, object_marking_refs) OR invalid object relationships | easy |
| performing-asset-criticality-scoring-for-vulns | vuln.asset.criticality-weighted-score | json-config | Apply fixed weighted formula (data sensitivity 0.25, business function 0.20, regulatory scope 0.15, etc.) to compute asset criticality score | easy |
| performing-cve-prioritization-with-kev-catalog | vuln.cve.known-exploited-prioritization | json-config | Cross-reference discovered CVEs against CISA Known Exploited Vulnerabilities (KEV) catalog to flag actively exploited vulnerabilities for prioritized remediation | easy |
| performing-web-application-vulnerability-triage | vuln.severity-classification | json-config | Deduplicate and prioritize vulnerabilities by CVSS score and assign SLA deadlines (7d critical, 30d high, 90d medium) | easy |
| implementing-vulnerability-sla-breach-alerting | vuln.sla.breach-detection | json-config | Detect vulnerabilities past SLA deadline or approaching within warning window; categorize by overdue days | easy |
| implementing-vulnerability-remediation-sla | vuln.sla.remediation-compliance | json-config | Check if vulnerability age exceeds SLA deadline based on severity tier; flag breached status | easy |
| performing-security-headers-audit | web.header.missing-security-header | http-headers | HSTS missing OR CSP absent/weak (unsafe-inline/unsafe-eval/wildcard) OR X-Frame-Options missing OR cookies lack Secure/HttpOnly/SameSite attributes | easy |
| exploiting-http-request-smuggling | web.http.content-length-transfer-encoding-desync | http-request | Detect Content-Length and Transfer-Encoding header conflicts that enable HTTP request smuggling | hard |
| exploiting-broken-link-hijacking | web.link.hijackable-domain | html-content | Match external links against patterns for expired domains (GitHub, npm, PyPI, Twitter, GitLab) that return 404 | easy |
| performing-web-application-penetration-test | web.missing-security-headers | http-headers | HTTP response missing security headers (HSTS, CSP, X-Content-Type-Options, X-Frame-Options) | easy |
| exploiting-server-side-request-forgery | web.ssrf.internal-metadata-access | http-request | Match URL parameters against SSRF bypass patterns (localhost, cloud metadata, file://) to access internal resources | easy |
| implementing-google-workspace-phishing-protection | workspace.gmail.phishing-protection-enabled | json-config | Check Gmail safety settings are enabled (spoofing, name spoofing, enhanced scanning) | easy |
| implementing-google-workspace-sso-configuration | workspace.sso.config-validation | tls-config | Validate SSO enabled and cert HTTPS, parse SAML cert expiry and algorithm strength | medium |

## T2 (137)

| skill | rule_id | input | predicate | conv |
|---|---|---|---|---|
| hunting-for-data-staging-before-exfiltration |  | log-line | Archive tool execution with compression args or large file consolidation patterns | medium |
| hunting-for-dcom-lateral-movement |  | log-line | MMC20/ShellWindows/ShellBrowserWindow COM objects spawning suspicious children via Sysmon events | medium |
| hunting-for-dcsync-attacks |  | log-line | Non-DC accounts requesting DS-Replication GUID access with control-access rights via Event 4662 | medium |
| hunting-for-dns-tunneling-with-zeek |  | log-line | High-entropy subdomains, excessive query volume, or unusual TLS/NULL/CNAME record types in DNS queries | medium |
| hunting-for-domain-fronting-c2-traffic |  | log-line | SNI vs HTTP Host header domain root mismatch, especially CDN-backed SNI with non-CDN Host | medium |
| hunting-for-lateral-movement-via-wmi |  | log-line | WmiPrvSE.exe spawning cmd/powershell or WMI event subscriptions with suspicious patterns | medium |
| hunting-for-lolbins-execution-in-endpoint-logs |  | log-line | LOLBin execution (certutil, regsvr32, rundll32, etc.) with suspicious args or download patterns | medium |
| hunting-for-ntlm-relay-attacks |  | log-line | Event 4624 logon type 3 with NTLMSSP where WorkstationName IP mismatch or rapid multi-host auth | medium |
| hunting-for-persistence-mechanisms-in-windows |  | json-config | Registry Run keys, Winlogon, IFEO, or shell extensions containing LOLBins or temp-directory paths | medium |
| hunting-for-persistence-via-wmi-subscriptions |  | log-line | WMI EventFilter, CommandLineEventConsumer, or FilterToConsumerBinding with PowerShell/cmd patterns | medium |
| hunting-for-process-injection-techniques |  | log-line | Sysmon Event 8 (CreateRemoteThread) or Event 10 (ProcessAccess) with dangerous access rights from suspicious sources | medium |
| hunting-for-registry-persistence-mechanisms |  | json-config | Run keys, Winlogon, IFEO, COM hijacking paths with suspicious executable or obfuscated content | medium |
| hunting-for-registry-run-key-persistence |  | log-line | Sysmon Event 13 registry modifications to Run/RunOnce keys with temp paths, LOLBins, or encoded commands | medium |
| hunting-for-scheduled-task-persistence |  | log-line | Scheduled task actions containing PowerShell, cmd, LOLBins, or temp directory paths | medium |
| hunting-for-shadow-copy-deletion |  | log-line | vssadmin/wmic/PowerShell commands deleting shadow copies or disabling recovery mechanisms | medium |
| hunting-for-spearphishing-indicators |  | json-config | Email attachments with executable/macro extensions, phishing URLs, or urgency keywords | medium |
| hunting-for-suspicious-scheduled-tasks |  | log-line | Scheduled task actions with PowerShell encode flags, LOLBins, HTTP URLs, or temp paths | medium |
| hunting-for-t1098-account-manipulation |  | log-line | Event 4738/4728/4732/4756/5136 showing privilege-group additions or sensitive AD attribute modifications | medium |
| hunting-for-unusual-network-connections |  | log-line | Outbound connections to non-standard ports (not in COMMON_PORTS), rare destinations, or unusual frequency | medium |
| hunting-for-unusual-service-installations |  | log-line | Event 7045 service installation with temp paths, PowerShell execution, encoded commands, or LOLBins | medium |
| hunting-for-webshell-activity |  | log-line | Web server process spawning cmd/shell, or HTTP requests with cmd/exec/shell parameters to .asp/.php/.jsp files | medium |
| detecting-model-extraction-attacks | ai.inference.extraction-attack-scoring | json-inference-audit-log | Scores principals on query volume (high count), input uniqueness (low diversity = boundary probing), confidence exposure (wants_probs flag); flags high-extraction-risk patterns | easy |
| detecting-ai-model-prompt-injection-attacks | ai.llm.prompt-injection-detection | source-code | User input matches known prompt injection regex patterns or scores high on heuristic anomaly analysis | medium |
| detecting-indirect-prompt-injection | ai.llm.prompt-injection-detector | text/html/pdf/image | Detects injection keywords/patterns in normalized text (zero-width/Unicode-tag stripped, Base64/ROT13 decoded) via regex heuristics; optional LLM Guard + ML model scoring | medium |
| detecting-data-and-model-poisoning | ai.model.poisoning-detection | model-artifact | Detect unsafe model serialization formats, hash mismatches, or poisoned training samples via activation clustering analysis | hard |
| implementing-api-abuse-detection-with-rate-limiting | api.abuse.brute-force-detection | log-line | Detect brute force/credential stuffing by counting auth failures and failed login attempts per client IP | medium |
| detecting-broken-object-property-level-authorization | api.bopla.sensitive-property-exposure | json-config | Detect JSON response properties containing sensitive keywords (password, ssn, credit_card, salary, etc.) | easy |
| performing-api-rate-limiting-bypass | api.bypass.rate-limit-header-detection | http-headers | Parse HTTP response headers to detect rate-limit info (X-RateLimit-*, Retry-After, etc.) and test bypass headers | easy |
| performing-api-inventory-and-discovery | api.discovery.endpoint-status-anomaly | http-headers | Probe common API paths via HTTP GET and detect active endpoints by status code, Content-Type headers, and schema presence | medium |
| analyzing-api-gateway-access-logs | api.gateway.bola-idor-detection | http-access-logs | Single user/IP accesses >50 unique resource IDs or >100 401/403 responses in time window | easy |
| exploiting-api-injection-vulnerabilities | api.injection.payload-test | http-request | Match SQL/NoSQL/OS/LDAP injection payloads against API responses for error indicators | medium |
| implementing-api-security-posture-management | api.posture.discovery-and-scoring | log-line | Discover APIs from traffic, classify by sensitivity (PII/Financial/Auth/Admin), score risk by error rate/consumer count | hard |
| exploiting-excessive-data-exposure-in-api | api.response.pii-leakage | json-response | Match JSON response fields against sensitive field names and PII regex patterns (email, SSN, credit card, passwords) | medium |
| detecting-api-enumeration-attacks | api.rest.sequential-id-enumeration | log-line | API requests show sequential numeric ID access pattern or high 404 error rate | easy |
| implementing-runtime-application-self-protection | app.rasp-coverage-check | json-config | RASP detection plugins for OWASP Top 10 (SQLi, CMDi, XSS, etc.) are disabled or in monitor-only mode | medium |
| testing-for-json-web-token-vulnerabilities | auth.jwt.weak-secret-or-alg-none | jwt-token | JWT uses algorithm=none, empty key, or weak HMAC secret from common wordlist | easy |
| detecting-anomalous-authentication-patterns | auth.login.impossible-travel | log-line | User login from geographically distant location in time insufficient for travel | medium |
| detecting-oauth-token-theft | auth.oauth.token-theft-anomaly | sign-in-logs-json | Detects impossible travel (haversine distance >900 km/h); flags new device first logons; monitors token replay from unusual IPs; detects overly broad scope requests | medium |
| exploiting-type-juggling-vulnerabilities | auth.php.type-juggling-bypass | http-headers | Detect PHP type juggling auth bypass via magic hash and type coercion payloads | easy |
| analyzing-azure-activity-logs-for-threats | cloud.azure.suspicious-admin-operations | azure-activity-json | Detects role assignments, keyvault access, NSG changes from anomalous sources/times | medium |
| analyzing-cloud-storage-access-patterns | cloud.storage.access-anomaly-detection | cloudtrail-json | Detects bulk downloads (>100 GetObjects/hour), after-hours access, new source IPs vs baseline | medium |
| detecting-container-escape-attempts | container.escape.vector-detection | falco-alert-json | Detect container escape via Falco alerts matching escape vectors (nsenter, unshare, mount, modprobe) and sensitive path access | easy |
| exploiting-nosql-injection-vulnerabilities | db.nosql.operator-injection | http-request | Detect MongoDB/NoSQL operator injection ($ne, $gt, $exists, $regex) in query parameters and response changes | medium |
| implementing-siem-use-case-tuning | detect.alert-false-positive-scorer | log-line | Alert rule has >50% false positive disposition rate OR generates >1000 alerts/day with <10% true positive disposition | easy |
| implementing-continuous-security-validation-with-bas | detect.attack.mitre-technique-execution | log-line | Detect emulated ATT&CK technique payload execution and scoring by prevention/detection/missed status | hard |
| detecting-deepfake-audio-in-vishing-attacks | detect.audio.deepfake-vishing | audio-file | Detect deepfake audio using spectral features (MFCC, spectral centroid, ZCR) and ML classification | hard |
| hunting-credential-stuffing-attacks | detect.auth.credential-stuffing | log-line | Detect credential stuffing via login velocity and account enumeration thresholds | easy |
| analyzing-command-and-control-communication | detect.c2.beacon-pattern-analysis | pcap-traffic | Detect periodic beacon intervals, DNS tunneling entropy, custom protocol anomalies in traffic | medium |
| hunting-for-command-and-control-beaconing | detect.c2.beaconing-pattern-detection | log-line | Detect C2 beaconing in DNS/HTTP traffic via frequency anomalies and suspicious patterns | medium |
| hunting-for-cobalt-strike-beacons | detect.c2.cobalt-strike-tls-signature | log-line | Detect Cobalt Strike beacons via TLS certificate serials and JA3/JARM hashes | easy |
| detecting-command-and-control-over-dns | detect.dns.c2-tunneling | zeek-dns-log | Detect DNS C2 using entropy analysis, beaconing detection, DGA classification via statistical/ML methods | hard |
| detecting-dns-exfiltration-with-dns-query-analysis | detect.dns.data-exfiltration | zeek-dns-log | Detect DNS exfiltration via high-entropy subdomains, excessive query length, abnormal TXT record usage, and volume spikes | medium |
| detecting-exfiltration-over-dns-with-zeek | detect.dns.exfiltration-zeek | zeek-dns-log | Detect DNS exfiltration using Shannon entropy analysis, abnormal subdomain length, and unique subdomain volume per domain | easy |
| analyzing-dns-logs-for-exfiltration | detect.dns.tunneling-and-dga | dns-query-logs | Subdomain length >50 chars with >20 queries, or SLD entropy >3.5 with length >12 chars | easy |
| performing-dns-tunneling-detection | detect.dns.tunneling-exfiltration | log-line | Detect DNS tunneling via Shannon entropy analysis of DNS query names, high subdomain cardinality, or abnormal query length distributions | medium |
| analyzing-typosquatting-domains-with-dnstwist | detect.domain-permutation-variants | none | Generate domain permutations and detect registration status | medium |
| analyzing-tls-certificate-transparency-logs | detect.domain-typosquatting-levenshtein | none | Detect typosquatting via Levenshtein distance from target domain | easy |
| detecting-business-email-compromise | detect.email.bec-header-spoofing | log-line | Detect BEC via SPF/DKIM/DMARC failures, display name spoofing, reply-to domain mismatch, urgency language patterns | easy |
| detecting-business-email-compromise-with-ai | detect.email.bec-nlp-features | log-line | Calculate BEC probability from NLP features: urgency, pressure, financial, authority keywords and writing style metrics | medium |
| investigating-phishing-email-incident | detect.email.phishing-indicators | http-headers | Extract and score phishing signals: parse SPF/DKIM/DMARC auth results; extract URLs via regex; flag suspicious originating IP or header mismatches | medium |
| performing-log-analysis-for-forensic-investigation | detect.forensic.event-sequence-correlation | log-line | 4624/4625/4648/4688 event sequence patterns establish compromise timeline | hard |
| detecting-insider-data-exfiltration-via-dlp | detect.insider.data-volume-anomaly | csv-activity-logs | Flags users with file transfer volume >3 std-dev above baseline; detects off-hours (6pm-6am) access with >10 events; identifies unusual upload destinations | medium |
| investigating-insider-threat-indicators | detect.insider.exfil-indicators | log-line | Score insider threat via data movement patterns: bulk GB transfers, personal cloud storage targets, off-hours/weekend activity anomaly thresholds | medium |
| detecting-insider-threat-behaviors | detect.insider.threat-indicators | json-activity-logs | Scores user behavior against risk indicators (off-hours access, mass downloads, privilege escalation, cloud/personal domain uploads, resignation correlation) | medium |
| performing-malware-ioc-extraction | detect.ioc.extract-from-text | none | Extract IPv4, IPv6, domains, URLs, emails, hashes, CVEs, file paths using regex patterns | easy |
| collecting-indicators-of-compromise | detect.ioc.regex-pattern-scoring | log-line | Extract and score IOC patterns (IPv4, domain, SHA256, MD5, URL) via regex and API enrichment confidence | medium |
| performing-lateral-movement-detection | detect.lateral-movement.pass-the-hash | log-line | NTLM Type 3 logon from single source to multiple targets indicates pass-the-hash | medium |
| performing-linux-log-forensics-investigation | detect.linux-auth.brute-force-attempt | log-line | 5+ failed login attempts from same IP within time window indicates brute force | medium |
| analyzing-linux-system-artifacts | detect.linux.persistence-and-compromise-artifacts | linux-artifacts-config | Detect suspicious patterns in /etc/passwd (UID 0), weak hashes, cron jobs, SSH keys, and system configs | medium |
| analyzing-persistence-mechanisms-in-linux | detect.linux.persistence-mechanisms | linux-config-files | Detect suspicious cron/systemd/LD_PRELOAD entries via pattern matching: reverse shells, privilege escalation | easy |
| analyzing-macro-malware-in-office-documents | detect.malware.office-macro-indicators | office-document | Detect malicious VBA patterns: download cradles, obfuscation, API calls (WebClient, Invoke-Expression) | hard |
| analyzing-powershell-empire-artifacts | detect.malware.powershell-empire-framework | windows-event-log | Detect Empire launcher/stager regex patterns in event logs: WebClient, Base64, Invoke-Expression | easy |
| analyzing-powershell-script-block-logging | detect.malware.powershell-obfuscation-and-execution | evtx-file | Parse Event 4104 EVTX; detect obfuscation (entropy), suspicious patterns (AMSI bypass, credential access) | medium |
| analyzing-ransomware-encryption-mechanisms | detect.malware.ransomware-encryption-indicators | binary-file | Identify ransomware via extension matching, entropy analysis, and embedded crypto constants/API names | easy |
| analyzing-malware-sandbox-evasion-techniques | detect.malware.sandbox-evasion-patterns | cuckoo-report-json | Detect evasion APIs: timing checks (GetTickCount), sleep inflation, VM registry checks, WMI queries | easy |
| analyzing-network-flow-data-with-netflow | detect.network.c2-and-exfiltration | netflow-json | Detect port scanning (>=20 unique ports), data exfiltration (threshold bytes), beaconing patterns | easy |
| detecting-beaconing-patterns-with-zeek | detect.network.c2-beaconing | zeek-conn-log | Detect periodic connections with low coefficient of variation in inter-arrival times using statistical analysis | medium |
| hunting-for-beaconing-with-frequency-analysis | detect.network.c2-beaconing-frequency | log-line | Detect C2 beaconing via periodic connection intervals and jitter coefficient | easy |
| analyzing-network-traffic-of-malware | detect.network.c2-traffic-signatures | pcap-file | Extract TCP streams, analyze payload entropy, detect C2 protocols and exfiltration patterns | easy |
| analyzing-network-covert-channels-in-malware | detect.network.covert-channels | pcap-file | Detect DNS tunneling (entropy >3.5), ICMP covert channels (high-entropy payload), protocol abuse patterns | medium |
| hunting-for-data-exfiltration-indicators | detect.network.data-exfiltration | log-line | Detect data exfiltration via DNS entropy, large uploads, and suspicious port usage | easy |
| analyzing-network-packets-with-scapy | detect.network.malicious-packet-patterns | pcap-file | Analyze PCAP: extract flows, compute traffic stats, detect anomalies in packet patterns and flags | easy |
| performing-s7comm-protocol-security-analysis | detect.plc.multiple-sources | pcap | Multiple unique source IPs accessing same PLC (threshold > 3) OR dangerous S7comm function codes detected (write/run/stop) OR repeated failed auth attempts (brute-force pattern) | medium |
| hunting-for-anomalous-powershell-execution | detect.powershell.anomalous-execution | evtx | Detect obfuscated/malicious PowerShell from event logs via keyword and pattern matching | medium |
| implementing-privileged-session-monitoring | detect.privaccess.policy-violation | log-line | Detect privileged sessions violating policy: duration >8h, idle >30m, outside 6-22h, or executing restricted commands (rm -rf, shutdown, passwd root, etc.) | medium |
| investigating-ransomware-attack-artifacts | detect.ransomware.variant-identification | log-line | Identify ransomware variant via ransom note IOC extraction: regex-match bitcoin/monero/tor/email patterns; match encrypted file extensions to known families | medium |
| performing-scada-hmi-security-assessment | detect.scada.insecure-protocol | pcap | SCADA ports (Modbus/S7comm/DNP3/OPC-UA) open OR unencrypted SCADA protocols detected in traffic OR default credentials accepted on HMI interface | medium |
| implementing-network-traffic-baselining | detect.traffic.baseline-anomaly | log-line | Detect traffic deviating >2 std devs from hourly/host baseline; score by volume, port entropy, protocol distribution | medium |
| analyzing-web-server-logs-for-intrusion | detect.web-sql-injection-pattern | log-line | Detect SQL injection via regex patterns | easy |
| performing-soap-web-service-security-testing | detect.webservice.xxe-vulnerability | http-headers | WSDL endpoint accepts XXE payloads OR XXE external entity references resolve OR SOAP error responses disclose internal paths/SQL errors OR WS-Security policy enforcement missing | hard |
| performing-lateral-movement-with-wmiexec | detect.windows-event.wmi-lateral-movement | log-line | wmiprvse.exe parent + network logon type 3 + service creation indicates WMI lateral movement | medium |
| implementing-siem-correlation-rules-for-apt | detect.windows-lateral-movement-chain | log-line | Sequence of Windows events (4624 RDP, 4688 process create, 7045 service install) within 15-min sliding window from same source | medium |
| analyzing-malware-persistence-with-autoruns | detect.windows.autoruns-persistence-anomaly | autoruns-csv | Identify suspicious persistence entries: LOLBins, unsigned binaries, suspicious paths, VirusTotal detections | easy |
| detecting-qr-code-phishing-with-email-security | email.qr-code-phishing-risk | email-eml | Score QR codes extracted from email images for phishing risk based on decoded URL indicators | medium |
| hardening-linux-endpoint-with-cis-benchmark | endpoint.linux-cis-hardening | log-line | Score Linux endpoint configuration against CIS Benchmark (filesystem, services, PAM) | hard |
| hardening-windows-endpoint-with-cis-benchmark | endpoint.windows-cis-hardening | log-line | Score Windows endpoint policies against CIS Benchmark (passwords, audit, GPO) | hard |
| implementing-gdpr-data-subject-access-request | gdpr.dsar.pii-detection | source-code|log-line | Detect PII patterns (email, phone, IBAN, passport, SSN) via regex across text/logs | easy |
| building-role-mining-for-rbac-optimization | iam.rbac.role-clustering-score | json-config | Score/cluster user-permission matrix to find optimal minimal RBAC roles | hard |
| performing-steganography-detection | image.lsb-anomaly-detection | image-file | Image LSB ratio deviates significantly from 0.5 (>0.55 or <0.45) indicating potential data hiding | medium |
| performing-insider-threat-investigation | insider-threat.user-behavior.after-hours-activity | log-line | Detect after-hours user activity anomalies vs baseline behavior from CSV logs | easy |
| analyzing-linux-audit-logs-for-intrusion | linux.auditd.intrusion-detection-patterns | auditd-log-lines | Detects /etc/passwd modifications, privilege escalation syscalls, unauthorized SSH key access | medium |
| implementing-llm-guardrails-for-security | llm.guardrails.prompt-injection-pii-detection | log-line | Detect prompt injection, PII, blocked topics, and harmful requests via regex and content policy rules | medium |
| reverse-engineering-ransomware-encryption-routine | malware.binary.crypto-algorithm-detection | source-code | Scan binary for AES S-box, RSA markers, ChaCha20/Salsa20 constants; flag if detected (indicates hybrid encryption) | medium |
| analyzing-cobaltstrike-malleable-c2-profiles | malware.cobalt-strike.malleable-c2-indicator-extraction | c2-profile-text | Parse malleable C2 profile DSL to extract HTTP URIs, headers, user agents, process injection config | medium |
| deobfuscating-javascript-malware | malware.js.obfuscation-detection | source-code | JavaScript code contains hex encoding, Unicode escapes, atob/unescape calls, eval chains, or charCode arrays | easy |
| implementing-ransomware-kill-switch-detection | malware.mutex-kill-switch-detection | source-code | log-line | Detects known ransomware mutex names, kill switch domains, or registry markers via pattern matching against hardcoded IOC list | easy |
| performing-threat-hunting-with-yara-rules | malware.pattern-matching | binary-file | Binary/file matches YARA rules for suspicious PowerShell patterns, Mimikatz strings, or webshell signatures | medium |
| performing-vlan-hopping-attack | net.layer2-dtp-detection | network-packet | Network packet contains DTP frames (LLC SNAP) or 802.1Q double-tagged frames indicating VLAN trunk negotiation | hard |
| detecting-shadow-it-cloud-usage | net.shadow-it-saas-domain | log-line | Classify traffic domains against known SaaS/dangerous categories, score as shadow IT | medium |
| detecting-dnp3-protocol-anomalies | ot.dnp3.unauthorized-commands | zeek-dnp3-log | Detect unauthorized DNP3 control commands (OPERATE, DIRECT_OPERATE, RESTART) from unknown sources in SCADA traffic | medium |
| monitoring-scada-modbus-traffic-anomalies | ot.modbus.unauthorized-write-detect | log-line | Detect Modbus write function codes (5,6,15,16) from unauthorized source IPs or frequency anomalies vs baseline | medium |
| detecting-anomalies-in-industrial-control-systems | ot.scada.traffic-anomaly | log-line | Industrial protocol traffic (Modbus/DNP3/OPC UA) deviates from baseline by >threshold | hard |
| performing-supply-chain-attack-simulation | package.typosquatting-detection | package-manifest | Package name has Levenshtein distance <= 2 from popular PyPI packages | easy |
| analyzing-certificate-transparency-for-phishing | phishing.ct-logs.lookalike-domain-detection | certificate-transparency-json | Domain certificate registered with >60% Levenshtein similarity to monitored domain | medium |
| detecting-typosquatting-packages | pkg.typosquat-detection | package-manifest | Generate typo mutations, check against popular package corpus using Levenshtein distance | medium |
| performing-false-positive-reduction-in-siem | siem.alert.false-positive-pattern-detection | log-line | Analyze SIEM alert CSV for false positive patterns and noisy rules via counting and frequency analysis | easy |
| performing-indicator-lifecycle-management | threat-intel.ioc.lifecycle-expiration | log-line | Extract IOCs from text via regex patterns and check age against TTL policy | easy |
| performing-ai-driven-osint-correlation | threat-intel.osint.correlation-scorer | http-headers | Score and correlate OSINT indicators (IPs, domains, email patterns) from multiple sources using frequency analysis and regex matching | hard |
| analyzing-apt-group-with-mitre-navigator | threat.intel.apt-ttp-coverage-mapping | mitre-attack-json | Query ATT&CK data and generate layer JSON with TTP scores for APT group | medium |
| implementing-threat-modeling-with-mitre-attack | threat.mitre-attack-coverage-mapping | none | Organizational security controls do not provide detection or mitigation for threat group TTPs ranked high by industry/region | hard |
| implementing-siem-use-cases-for-detection | threat.mitre-coverage-gap | none | Organizational SIEM detections cover <70% of ATT&CK techniques used by industry threat groups | medium |
| prioritizing-vulnerabilities-with-cvss-scoring | vuln.cvss.score-calculation | json-config | Parse CVSS vector string and compute v3.1 base score; flag if score >= 7.0 (high/critical) | medium |
| performing-web-cache-deception-attack | web.cache-deception-path-confusion | http-response | Authenticated content served with cache headers (X-Cache: HIT) via path confusion with static extensions | medium |
| performing-web-cache-poisoning-attack | web.cache-layer-detection | http-headers | HTTP response headers identify CDN/cache layer (Cloudflare, Varnish, Akamai) and unkeyed headers influence cache | medium |
| performing-clickjacking-attack-test | web.http.clickjacking-missing-frameopt | http-response-headers | Detect missing X-Frame-Options and CSP frame-ancestors directives in HTTP response headers | medium |
| performing-content-security-policy-bypass | web.http.csp-misconfiguration | http-response-headers | Detect overly permissive CSP directives (unsafe-inline, unsafe-eval, wildcard sources) or policy injection vectors in Content-Security-Policy header | medium |
| exploiting-race-condition-vulnerabilities | web.race-condition.toctou-timing | http-request | Detect time-of-check-to-time-of-use flaws via concurrent request analysis for balance/inventory changes | hard |
| detecting-sql-injection-via-waf-logs | web.sql-injection-patterns | log-line | Match SQL injection patterns (UNION, SLEEP, LOAD_FILE, etc) in WAF logs | easy |
| performing-ssrf-vulnerability-exploitation | web.ssrf-metadata-endpoints | http-response | HTTP 200 response with metadata endpoint indicators (ami-id, access_token, compute) from 169.254.169.254 or metadata.google.internal | easy |
| implementing-browser-isolation-for-zero-trust | web.url.risk-categorization | http-headers | Categorize URLs by risk weight and apply browser isolation policies based on domain/pattern matching | medium |
| performing-web-application-firewall-bypass | web.waf-bypass-encoding | http-request | HTTP request passes WAF-blocked content via encoding (URL, hex entities, unicode fullwidth, null bytes) techniques | hard |
| detecting-dcsync-attack-in-active-directory | windows.ad.dcsync-attack | log-line | Detect DS-Replication-Get-Changes operations (GUIDs 1131f6aa-9c07-11d1-f79f-00c04fc2dcd2, etc.) from non-DC accounts via Event 4662 | medium |
| detecting-credential-dumping-techniques | windows.credential-dumping.lsass-access | log-line | Detect LSASS credential dumping via suspicious access codes (0x1010, 0x1410, 0x1FFFFF), comsvcs MiniDump patterns, SAM export commands | easy |
| detecting-evasion-techniques-in-endpoint-logs | windows.defense-evasion.evasion-indicators | log-line | Detect defense evasion: log clearing (wevtutil, Clear-EventLog), timestomping, process injection, security tool disabling via regex patterns | easy |
| detecting-dll-sideloading-attacks | windows.dll.sideloading-detection | log-line | Detect unsigned DLLs loaded by signed executables from non-standard paths matching known sideload targets via Sysmon Event ID 7 | medium |
| detecting-fileless-attacks-on-endpoints | windows.fileless.attack-indicators | log-line | Detect fileless attacks via PowerShell script block analysis (Event 4104), WMI persistence events, memory injection indicators | easy |
| detecting-golden-ticket-attacks-in-kerberos-logs | windows.kerberos.golden-ticket | log-line | Detect forged Kerberos TGTs via anomalous encryption types (RC4 vs AES), impossible lifetimes, non-existent accounts, no TGT request preceding TGS | medium |

## ADAPTER (399)

| skill | external_tool |
|---|---|
| exploiting-sql-injection-with-sqlmap | sqlmap |
| exploiting-template-injection-vulnerabilities | tplmap/SSTImap |
| exploiting-vulnerabilities-with-metasploit-framework | metasploit |
| exploiting-zerologon-vulnerability-cve-2020-1472 | nmap/impacket |
| extracting-credentials-from-memory-dump | volatility3/pypykatz |
| extracting-memory-artifacts-with-rekall | rekall |
| fleet-hunting-with-velociraptor | velociraptor |
| generating-and-analyzing-sboms | syft/grype/cosign |
| generating-forensic-timelines-with-hayabusa | hayabusa |
| hunting-evtx-with-chainsaw | chainsaw |
| hunting-for-dns-based-persistence | SecurityTrails-API |
| hunting-for-living-off-the-cloud-techniques | Elasticsearch |
| hunting-for-living-off-the-land-binaries | Elasticsearch |
| hunting-saas-sso-token-abuse | Okta-API |
| implementing-alert-fatigue-reduction | Splunk-SDK |
| detecting-ransomware-precursors-in-network | powershell,tasklist,ps |
| detecting-rootkit-activity | volatility,rkhunter,chkrootkit |
| detecting-s3-data-exfiltration-attempts | guardduty,cloudtrail,s3api,macie |
| detecting-secure-boot-bypass | mokutil,efi-readvar,dbxtool,chipsec |
| detecting-serverless-function-injection | aws-lambda-api,cloudtrail,boto3 |
| detecting-service-account-abuse | powershell,active-directory |
| detecting-suspicious-oauth-application-consent | microsoft-graph-api,msal |
| detecting-suspicious-powershell-execution | powershell,windows-event-logs |
| detecting-t1003-credential-dumping-with-edr | sysmon,powershell |
| detecting-t1055-process-injection-with-sysmon | sysmon |
| detecting-t1548-abuse-elevation-control-mechanism | powershell,windows-registry |
| detecting-typosquatting-packages-in-npm-pypi | npm-registry,pypi-registry |
| detecting-wmi-persistence | sysmon |
| emulating-cloud-attacks-with-stratus-red-team | stratus-red-team |
| enumerating-cloud-with-cloudfox | cloudfox |
| executing-active-directory-attack-simulation | active-directory,impacket,ldap3 |
| auditing-kubernetes-rbac-privilege-escalation | kubectl |
| auditing-mcp-servers-for-tool-poisoning | mcp-scan |
| auditing-terraform-infrastructure-for-security | checkov, tfsec, terrascan |
| auditing-tls-certificate-transparency-logs | crt.sh API |
| auditing-uefi-firmware-with-chipsec | chipsec |
| automating-ioc-enrichment | virustotal, abuseipdb, shodan |
| benchmarking-kubernetes-with-kube-bench | kube-bench |
| building-adversary-infrastructure-tracking-system | passive-dns, whois APIs |
| building-automated-malware-submission-pipeline | cuckoo, virustotal, any.run |
| conducting-memory-forensics-with-volatility | volatility |
| conducting-mobile-app-penetration-test | apktool, adb, frida, MobSF |
| conducting-network-penetration-test | nmap |
| conducting-wireless-network-penetration-test | aircrack-ng, kismet |
| configuring-host-based-intrusion-detection | wazuh, osquery |
| configuring-pfsense-firewall-rules | pfsense-api |
| building-super-timelines-with-plaso | plaso/log2timeline |
| building-vulnerability-scanning-workflow | nessus/nmap |
| bypassing-authentication-with-forced-browsing | ffuf/gobuster |
| coercing-authentication-with-coercer-petitpotam | coercer/petitpotam |
| collecting-open-source-intelligence | shodan/crt.sh |
| conducting-api-security-testing | burp-suite/live-endpoint-testing |
| conducting-cloud-incident-response | aws-cli/azure-cli/gcp-sdk |
| conducting-domain-persistence-with-dcsync | ldap3/impacket |
| conducting-internal-network-penetration-test | responder/impacket |
| conducting-internal-reconnaissance-with-bloodhound-ce | bloodhound |
| conducting-malware-incident-response | virustotal-api |
| conducting-man-in-the-middle-attack-simulation | ettercap/mitmproxy/bettercap |
| detecting-azure-storage-account-misconfigurations | azure-mgmt-storage |
| detecting-bluetooth-low-energy-attacks | bleak, ubertooth-btle, tshark, crackle |
| detecting-cloud-threats-with-guardduty | guardduty |
| detecting-compromised-cloud-credentials | guardduty, cloudtrail |
| detecting-container-drift-at-runtime | docker |
| detecting-container-escape-with-falco-rules | falco |
| detecting-container-runtime-threats-with-falco | falco |
| detecting-cryptomining-in-cloud | guardduty |
| detecting-email-account-compromise | microsoft-graph |
| detecting-email-forwarding-rules-attack | microsoft-graph |
| detecting-entra-offensive-tools-in-graph-logs | azure-monitor-api |
| detecting-fileless-malware-techniques | volatility |
| executing-phishing-simulation-campaign | GoPhish |
| exploiting-active-directory-certificate-services-esc1 | Certipy,certutil |
| exploiting-active-directory-with-bloodhound | BloodHound,SharpHound,bloodhound-python |
| exploiting-adcs-with-certipy | Certipy |
| exploiting-aws-with-pacu | Pacu |
| exploiting-constrained-delegation-abuse | PowerShell,Rubeus |
| exploiting-insecure-data-storage-in-mobile | apktool,strings,aapt |
| exploiting-kerberoasting-with-impacket | Impacket |
| exploiting-ms17-010-eternalblue-vulnerability | Metasploit |
| exploiting-nopac-cve-2021-42278-42287 | PowerShell |
| exploiting-smb-vulnerabilities-with-metasploit | Metasploit |
| implementing-gcp-organization-policy-constraints | gcloud |
| implementing-gcp-vpc-firewall-rules | google-cloud-compute-api |
| implementing-github-advanced-security-for-code-scanning | github-api |
| implementing-google-workspace-admin-security | google-admin-sdk |
| implementing-hashicorp-vault-dynamic-secrets | hashicorp-vault |
| implementing-honeytokens-for-breach-detection | canarytokens-api |
| implementing-identity-governance-with-sailpoint | sailpoint-api |
| implementing-image-provenance-verification-with-cosign | cosign |
| implementing-immutable-backup-with-restic | restic |
| implementing-infrastructure-as-code-security-scanning | checkov/tfsec |
| implementing-log-forwarding-with-fluentd | fluent-bit/fluentd |
| implementing-memory-protection-with-dep-aslr | bcdedit/powershell |
| implementing-microsegmentation-with-guardicore | guardicore-api |
| implementing-cloud-vulnerability-posture-management | Prowler, ScoutSuite, AWS Security Hub |
| implementing-cloud-workload-protection | AWS SSM |
| implementing-conditional-access-policies-azure-ad | Microsoft Graph API |
| implementing-container-image-minimal-base-with-distroless | Trivy |
| implementing-container-network-policies-with-calico | calicoctl, Kubernetes API |
| implementing-data-loss-prevention-with-microsoft-purview | Microsoft Graph DLP API |
| implementing-ddos-mitigation-with-cloudflare | Cloudflare API |
| implementing-deception-based-detection-with-canarytoken | Thinkst Canary API |
| implementing-delinea-secret-server-for-pam | Delinea Secret Server API |
| implementing-device-posture-assessment-in-zero-trust | OS platform API |
| implementing-devsecops-security-scanning | Semgrep, Trivy, Gitleaks |
| implementing-disk-encryption-with-bitlocker | manage-bde |
| implementing-dragos-platform-for-ot-monitoring | Dragos Platform API |
| implementing-ebpf-security-monitoring | kubectl, helm, Cilium Tetragon |
| implementing-email-sandboxing-with-proofpoint | Proofpoint TAP SIEM API |
| implementing-endpoint-detection-with-wazuh | Wazuh REST API |
| implementing-envelope-encryption-with-aws-kms | AWS KMS |
| implementing-epss-score-for-vulnerability-prioritization | FIRST EPSS API |
| implementing-file-integrity-monitoring-with-aide | AIDE |
| implementing-fuzz-testing-in-cicd-with-aflplusplus | AFL++ |
| implementing-gcp-binary-authorization | gcloud |
| analyzing-ransomware-payment-wallets | blockchain.com, blockstream APIs |
| analyzing-security-logs-with-splunk | Splunk Enterprise |
| analyzing-slack-space-and-file-system-artifacts | The Sleuth Kit, analyzeMFT |
| analyzing-threat-actor-ttps-with-mitre-navigator | TAXII 2.1 server, attackcti |
| analyzing-threat-intelligence-feeds | TAXII 2.1 server |
| analyzing-threat-landscape-with-misp | MISP |
| analyzing-windows-event-logs-in-splunk | Splunk Enterprise |
| assessing-vector-and-embedding-weaknesses | sentence-transformers, Qdrant vector store |
| attacking-entra-id-with-roadtools | roadrecon, roadtx binaries |
| attacking-oauth-with-device-code-phishing | Microsoft Entra ID OAuth endpoints |
| auditing-aws-s3-bucket-permissions | AWS S3 API |
| auditing-azure-active-directory-configuration | Microsoft Graph API |
| auditing-cloud-with-cis-benchmarks | AWS APIs |
| auditing-entra-id-with-aadinternals | AADInternals PowerShell |
| auditing-foundry-smart-contract-security | slither, aderyn, mythril, forge |
| auditing-gcp-iam-permissions | GCP IAM API |
| auditing-kubernetes-cluster-rbac | Kubernetes API |
| implementing-rapid7-insightvm-for-scanning | rapid7-insightvm |
| implementing-runtime-security-with-tetragon | kubernetes-api |
| implementing-scim-provisioning-with-okta | okta-api |
| implementing-secret-scanning-with-gitleaks | gitleaks |
| implementing-secrets-management-with-vault | hashicorp-vault |
| implementing-secrets-scanning-in-ci-cd | gitleaks / trufflehog |
| implementing-security-monitoring-with-datadog | datadog-api |
| implementing-semgrep-for-custom-sast-rules | semgrep |
| implementing-soar-automation-with-phantom | splunk-soar |
| implementing-soar-playbook-for-phishing | splunk-soar |
| implementing-soar-playbook-with-palo-alto-xsoar | cortex-xsoar |
| implementing-stix-taxii-feed-integration | taxii-server |
| implementing-supply-chain-security-with-in-toto | in-toto |
| implementing-taxii-server-with-opentaxii | opentaxii |
| implementing-threat-intelligence-lifecycle-management | virustotal / abuseipdb |
| implementing-ticketing-system-for-incidents | ServiceNow,TheHive |
| implementing-usb-device-control-policy | usbguard,lsusb,powershell |
| implementing-velociraptor-for-ir-collection | Velociraptor |
| implementing-vulnerability-management-with-greenbone | Greenbone/OpenVAS |
| implementing-web-application-logging-with-modsecurity | ModSecurity WAF |
| integrating-dast-with-owasp-zap-in-pipeline | OWASP ZAP |
| integrating-sast-into-github-actions-pipeline | Semgrep,CodeQL |
| mapping-attack-paths-with-bloodhound-ce | BloodHound CE |
| modeling-threats-with-opencti | OpenCTI |
| monitoring-darkweb-sources | dark web monitoring services (Recorded Future, Flashpoint, etc.) |
| correlating-security-events-in-qradar | IBM QRadar SIEM |
| defending-llms-with-guardrails | Llama Guard, NeMo Guardrails, LLM Guard |
| deploying-edr-agent-with-crowdstrike | CrowdStrike FalconPy |
| detecting-arp-poisoning-in-network-traffic | ARPWatch, Wireshark, Dynamic ARP Inspection |
| detecting-aws-cloudtrail-anomalies | AWS CloudTrail API (boto3) |
| detecting-aws-credential-exposure-with-trufflehog | TruffleHog |
| detecting-aws-guardduty-findings-automation | AWS GuardDuty, EventBridge, Lambda |
| detecting-aws-iam-privilege-escalation | boto3 IAM API, Cloudsplaining |
| detecting-azure-lateral-movement | Microsoft Graph API, Azure Sentinel KQL |
| detecting-azure-service-principal-abuse | Microsoft Graph API |
| abusing-dpapi-for-credential-access | SharpDPAPI, impacket-dpapi, DonPAPI |
| abusing-shadow-credentials-for-privesc | certipy, pyWhisker, PKINITtools |
| analyzing-android-malware-with-apktool | apktool, jadx, androguard |
| analyzing-bootkit-and-rootkit-samples | UEFITool, chipsec, Ghidra, Volatility3 |
| analyzing-browser-forensics-with-hindsight | Hindsight (pyhindsight) |
| analyzing-disk-image-with-autopsy | Autopsy, Sleuth Kit |
| analyzing-docker-container-forensics | docker, dive, container-diff |
| analyzing-ethereum-smart-contract-vulnerabilities | slither, mythril |
| analyzing-golang-malware-with-ghidra | Ghidra, GoResolver |
| analyzing-heap-spray-exploitation | Volatility3 |
| analyzing-indicators-of-compromise | VirusTotal, AbuseIPDB, MalwareBazaar APIs |
| analyzing-ios-app-security-with-objection | objection, Frida |
| analyzing-linux-elf-malware | strace, ltrace, GDB, readelf, objdump, Radare2, Ghidra |
| implementing-anti-ransomware-group-policy | PowerShell Get-AppLockerPolicy/Get-MpPreference |
| implementing-api-gateway-security-controls | AWS API Gateway SDK, Kong Admin API |
| implementing-aqua-security-for-container-scanning | trivy |
| implementing-attack-path-analysis-with-xm-cyber | XM Cyber API |
| implementing-attack-surface-management | Shodan, Censys, ProjectDiscovery (subfinder/httpx/nuclei) |
| implementing-aws-config-rules-for-compliance | boto3 AWS Config |
| implementing-aws-iam-permission-boundaries | boto3 AWS IAM |
| implementing-aws-macie-for-data-classification | boto3 AWS Macie |
| implementing-aws-nitro-enclave-security | boto3 AWS EC2/KMS |
| implementing-aws-security-hub-compliance | boto3 AWS Security Hub |
| implementing-aws-security-hub | boto3 AWS Security Hub |
| implementing-azure-ad-privileged-identity-management | Microsoft Graph API (requests) |
| implementing-azure-defender-for-cloud | Azure SDK (azure-mgmt-security) |
| implementing-beyondcorp-zero-trust-access-model | Google Cloud IAP API (requests) |
| implementing-bgp-security-with-rpki | RIPEstat API, Cloudflare RPKI API |
| implementing-canary-tokens-for-network-intrusion | Canarytokens.org API |
| implementing-cloud-dlp-for-data-protection | Google Cloud DLP API, boto3 AWS Macie |
| implementing-cloud-security-posture-management | prowler |
| implementing-cloud-trail-log-analysis | boto3 AWS CloudTrail |
| triaging-windows-with-kape | KAPE (kape.exe) |
| validating-tpm-measured-boot-attestation | tpm2-tools (tpm2_pcrread, tpm2_eventlog, tpm2_quote, tpm2_checkquote) |
| verifying-build-provenance-with-slsa-sigstore | cosign, slsa-verifier |
| performing-ssl-tls-inspection-configuration | openssl, firewall CLI |
| performing-ssl-tls-security-assessment | sslyze |
| performing-subdomain-enumeration-with-subfinder | subfinder, httpx |
| performing-threat-emulation-with-atomic-red-team | atomic-operator, atomic-red-team YAML |
| performing-threat-hunting-with-elastic-siem | elasticsearch, kibana |
| performing-threat-intelligence-sharing-with-misp | misp, pymisp |
| performing-timeline-reconstruction-with-plaso | log2timeline, psort |
| performing-user-behavior-analytics | elasticsearch, haversine distance calc |
| performing-vulnerability-scanning-with-nessus | nessus |
| performing-web-application-scanning-with-nikto | nikto |
| performing-wifi-password-cracking-with-aircrack | aircrack-ng, hashcat |
| performing-windows-artifact-analysis-with-eric-zimmerman-tools | MFTECmd, PECmd, LECmd, JLECmd |
| performing-wireless-network-penetration-test | aircrack-ng, airodump-ng |
| performing-wireless-security-assessment-with-kismet | kismet |
| post-exploiting-microsoft-graph-with-graphrunner | microsoft-graph-api |
| analyzing-linux-kernel-rootkits | volatility3, rkhunter |
| analyzing-malicious-pdf-with-peepdf | peepdf, pdfid, pdf-parser |
| analyzing-malicious-url-with-urlscan | URLScan.io API |
| analyzing-malware-behavior-with-cuckoo-sandbox | Cuckoo Sandbox |
| analyzing-malware-family-relationships-with-malpedia | Malpedia API |
| analyzing-memory-dumps-with-volatility | volatility3 |
| analyzing-memory-forensics-with-lime-and-volatility | LiME, volatility3 |
| analyzing-network-traffic-for-incidents | Wireshark, Zeek, NetFlow |
| analyzing-network-traffic-with-wireshark | Wireshark |
| analyzing-office365-audit-logs-for-compromise | Microsoft Graph API |
| analyzing-packed-malware-with-upx-unpacker | UPX unpacker |
| analyzing-pdf-malware-with-pdfid | pdfid |
| analyzing-ransomware-leak-site-intelligence | ransomware.live API, ransomlook.io API |
| analyzing-ransomware-network-indicators | Tor Project exit list API, Zeek/NetFlow |
| performing-docker-bench-security-assessment | docker-bench-security |
| performing-dynamic-analysis-of-android-app | frida, adb |
| performing-dynamic-analysis-with-any-run | ANY.RUN |
| performing-endpoint-forensics-investigation | wmic, netstat, volatility |
| performing-entitlement-review-with-sailpoint-iiq | SailPoint IdentityIQ REST API |
| performing-external-network-penetration-test | nmap |
| performing-file-carving-with-foremost | foremost, scalpel |
| performing-firmware-extraction-with-binwalk | binwalk |
| performing-firmware-malware-analysis | binwalk |
| performing-fuzzing-with-aflplusplus | aflplusplus |
| performing-gcp-penetration-testing-with-gcpbucketbrute | gcloud, gsutil |
| performing-gcp-security-assessment-with-forseti | GCP Cloud APIs (SCC, Asset Inventory) |
| performing-graphql-depth-limit-attack | live GraphQL endpoint |
| performing-graphql-introspection-attack | live GraphQL endpoint |
| performing-graphql-security-assessment | live GraphQL endpoint |
| performing-hardware-security-module-integration | HSM hardware device (PKCS#11) |
| performing-hash-cracking-with-hashcat | hashcat |
| performing-http-parameter-pollution-attack | live web application |
| performing-ics-asset-discovery-with-claroty | Claroty xDome REST API |
| performing-initial-access-with-evilginx3 | evilginx3 |
| performing-ioc-enrichment-automation | VirusTotal, AbuseIPDB, Shodan, GreyNoise APIs |
| performing-ios-app-security-assessment | frida, objection |
| performing-iot-security-assessment | nmap, binwalk, curl |
| performing-ip-reputation-analysis-with-shodan | Shodan API |
| performing-kerberoasting-attack | ldapsearch, impacket, powershell |
| profiling-threat-actor-groups | MITRE ATT&CK STIX (external data fetch) |
| recovering-deleted-files-with-photorec | PhotoRec |
| red-teaming-llms-with-garak | garak (NVIDIA LLM red-team scanner) |
| relaying-ntlm-for-adcs-esc8 | ntlmrelayx, certipy, PetitPotam/Coercer |
| reverse-engineering-android-malware-with-jadx | jadx, apktool, androguard |
| reverse-engineering-dotnet-malware-with-dnspy | dnSpy, de4dot, Detect It Easy (diec) |
| reverse-engineering-ios-app-with-frida | Frida |
| reverse-engineering-malware-with-ghidra | Ghidra (headless analysis) |
| reverse-engineering-rust-malware | Ghidra or IDA Pro (with Rust-specific analysis) |
| scanning-container-images-with-grype | Anchore Grype |
| scanning-containers-with-trivy-in-cicd | Aqua Trivy |
| scanning-docker-images-with-trivy | Aqua Trivy |
| scanning-iac-and-images-with-trivy | Aqua Trivy |
| scanning-infrastructure-with-nessus | Tenable Nessus (REST API) |
| scanning-kubernetes-manifests-with-kubesec | kubesec (REST API or CLI) |
| scanning-network-with-nmap-advanced | nmap |
| securing-azure-with-microsoft-defender | Microsoft Defender for Cloud (Azure CLI) |
| securing-container-registry-images | Trivy, Grype, Cosign, Syft |
| implementing-mimecast-targeted-attack-protection | Mimecast TAP API |
| implementing-mobile-application-management | Microsoft Intune MDM API |
| implementing-mtls-for-zero-trust-services | TLS endpoint |
| implementing-network-access-control-with-cisco-ise | Cisco ISE ERS API |
| implementing-network-access-control | RADIUS server |
| implementing-network-intrusion-prevention-with-suricata | Suricata IPS |
| implementing-network-segmentation-for-ot | nmap, firewall config file parsing |
| implementing-network-traffic-analysis-with-arkime | Arkime API |
| implementing-next-generation-firewall-with-palo-alto | Palo Alto Networks XML API |
| implementing-opa-gatekeeper-for-policy-enforcement | OPA (Open Policy Agent) binary |
| implementing-ot-network-traffic-analysis-with-nozomi | Nozomi Networks API |
| implementing-pam-for-database-access | PostgreSQL/MySQL/SQL Server CLI, openssl |
| implementing-passwordless-auth-with-microsoft-entra | Microsoft Graph API |
| implementing-passwordless-authentication-with-fido2 | Microsoft Graph API |
| implementing-patch-management-workflow | apt/yum package managers, CISA KEV API |
| implementing-pci-dss-compliance-controls | SSL/TLS endpoints |
| implementing-policy-as-code-with-open-policy-agent | OPA (Open Policy Agent) binary/REST API |
| implementing-privileged-access-management-with-cyberark | CyberArk PVWA REST API |
| implementing-privileged-access-workstation | PowerShell, Windows Registry |
| implementing-proofpoint-email-security-gateway | Proofpoint TAP SIEM API |
| performing-bandwidth-throttling-attack-simulation | tc (traffic control), iperf3, Scapy |
| performing-binary-exploitation-analysis | ROPgadget, pwntools (external binary analysis) |
| performing-blind-ssrf-exploitation | HTTP live interaction required |
| performing-bluetooth-security-assessment | Bluetooth hardware (bleak library) |
| performing-brand-monitoring-for-impersonation | dnstwist, DNS resolution, crt.sh API |
| performing-cloud-asset-inventory-with-cartography | Cartography (external cloud asset discovery), Neo4j database |
| performing-cloud-forensics-investigation | AWS CloudTrail API, EBS snapshots (live cloud APIs) |
| performing-cloud-forensics-with-aws-cloudtrail | AWS CloudTrail API, CloudTrail Lookup Events |
| performing-cloud-incident-containment-procedures | AWS/cloud APIs for resource modification (EC2, IAM, security groups) |
| performing-cloud-log-forensics-with-athena | AWS Athena (SQL query engine on CloudTrail logs) |
| performing-cloud-native-forensics-with-falco | Falco runtime monitoring engine (gRPC API) |
| performing-cloud-native-threat-hunting-with-aws-detective | AWS Detective API, behavior graphs |
| performing-cloud-penetration-testing-with-pacu | Pacu (AWS exploitation framework), AWS APIs |
| performing-cloud-storage-forensic-acquisition | AWS S3 API, Azure Blob Storage API, GCP Cloud Storage API |
| performing-container-image-hardening | Trivy vulnerability scanner, Dockle CIS benchmark checker |
| performing-container-security-scanning-with-trivy | Trivy (container image scanner) |
| performing-csrf-attack-simulation | HTTP live interaction with target web application |
| performing-dark-web-monitoring-for-threats | Have I Been Pwned API, paste sites, threat intelligence feeds |
| performing-deception-technology-deployment | Runtime honeypot/honeytoken server deployment |
| performing-directory-traversal-testing | HTTP live interaction with target web application |
| performing-disk-forensics-investigation | pytsk3 (filesystem parser), actual disk/image file required |
| performing-dns-enumeration-and-zone-transfer | DNS queries, zone transfer requests (network interaction) |
| performing-plc-firmware-security-analysis | binwalk |
| performing-post-quantum-cryptography-migration | openssl-oqs-provider |
| performing-privilege-escalation-assessment | find-sudo-getcap |
| performing-privilege-escalation-on-linux | find-grep-cat |
| performing-privileged-account-discovery | ldap3-activedirectory |
| performing-red-team-phishing-with-gophish | gophish-api |
| performing-red-team-with-covenant | covenant-c2 |
| performing-sca-dependency-scanning-with-snyk | snyk-cli |
| performing-service-account-audit | powershell-ldap-aws-cli |
| performing-service-account-credential-rotation | aws-cli-az-cli-vault |
| performing-ssl-stripping-attack | curl-bettercap |
| performing-kubernetes-cis-benchmark-with-kube-bench | kube-bench |
| performing-kubernetes-etcd-security-assessment | kubectl,etcdctl |
| performing-kubernetes-penetration-testing | kubectl,kube-hunter,kubescape,kube-bench |
| performing-malware-hash-enrichment-with-virustotal | VirusTotal API |
| performing-malware-triage-with-yara | yara |
| performing-memory-forensics-with-volatility3-plugins | volatility3 |
| performing-memory-forensics-with-volatility3 | volatility3 |
| performing-mobile-app-certificate-pinning-bypass | frida,apktool |
| performing-network-forensics-with-wireshark | pyshark,wireshark |
| performing-network-traffic-analysis-with-tshark | tshark,pyshark |
| performing-network-traffic-analysis-with-zeek | zeek |
| performing-oauth-scope-minimization-review | Microsoft Graph API |
| performing-open-source-intelligence-gathering | whois,dns.resolver |
| performing-osint-with-spiderfoot | SpiderFoot API |
| performing-ot-vulnerability-assessment-with-claroty | Claroty xDome API |
| performing-ot-vulnerability-scanning-safely | tshark,Tenable OT Security |
| performing-paste-site-monitoring-for-credentials | Have I Been Pwned API |
| securing-historian-server-in-ot-environment | socket |
| securing-kubernetes-on-cloud | kubernetes |
| securing-serverless-functions | boto3 |
| testing-android-intents-for-vulnerabilities | adb, drozer |
| testing-api-authentication-weaknesses | requests/HTTP |
| testing-api-for-broken-object-level-authorization | requests/HTTP |
| testing-api-for-mass-assignment-vulnerability | requests/HTTP |
| testing-api-security-with-owasp-top-10 | requests/HTTP |
| testing-cors-misconfiguration | requests/HTTP |
| testing-for-broken-access-control | requests/HTTP |
| testing-for-business-logic-vulnerabilities | requests/HTTP |
| testing-for-email-header-injection | requests/HTTP |
| testing-for-host-header-injection | requests/HTTP |
| testing-for-open-redirect-vulnerabilities | requests/HTTP |
| testing-for-sensitive-data-exposure | requests/HTTP |
| testing-for-system-prompt-leakage | OpenAI API |
| testing-for-xml-injection-vulnerabilities | requests/HTTP |
| testing-for-xss-vulnerabilities-with-burpsuite | requests/HTTP |
| testing-for-xss-vulnerabilities | requests/HTTP |
| testing-for-xxe-injection-vulnerabilities | requests/HTTP |
| testing-jwt-token-security | requests/HTTP |
| testing-mobile-api-authentication | requests/HTTP |
| testing-oauth2-implementation-flaws | requests/HTTP |
| testing-prompt-injection-in-rag-pipelines | requests/HTTP |
| testing-websocket-api-security | websockets |
| tracking-threat-actor-infrastructure | requests/HTTP (Shodan, VirusTotal, passive DNS) |
| triaging-security-alerts-in-splunk | splunklib |
| moving-laterally-with-netexec | NetExec (nxc) |
| operating-havoc-c2 | Havoc C2 Framework |
| operationalizing-misp-threat-feeds | MISP (via PyMISP) |
| orchestrating-llm-attacks-with-pyrit | PyRIT (Microsoft LLM Red-Team Framework) |
| parsing-artifacts-with-eric-zimmerman-tools | Eric Zimmerman Tools (MFTECmd, PECmd, etc.) |
| performing-access-recertification-with-saviynt | Saviynt Enterprise IAM |
| performing-active-directory-bloodhound-analysis | Neo4j (BloodHound database) |
| performing-active-directory-penetration-test | ldap3 (LDAP queries) + Impacket tools |
| performing-active-directory-vulnerability-assessment | Active Directory enumeration tools |
| performing-adversary-in-the-middle-phishing-detection | Email security services / DNS monitoring |
| performing-agentless-vulnerability-scanning | AWS boto3 + cloud APIs |
| performing-alert-triage-with-elastic-siem | Elasticsearch / Elastic Stack |
| performing-android-app-static-analysis-with-mobsf | MobSF (Mobile Security Framework) |
| performing-api-fuzzing-with-restler | RESTler API fuzzer |
| performing-api-security-testing-with-postman | Newman (Postman test runner) |
| performing-arp-spoofing-attack-simulation | Scapy (ARP packet crafting) |
| performing-authenticated-scan-with-openvas | GVM/OpenVAS vulnerability scanner |
| performing-authenticated-vulnerability-scan | Tenable Nessus |
| performing-automated-malware-analysis-with-cape | CAPE Sandbox |
| performing-aws-account-enumeration-with-scout-suite | ScoutSuite AWS security auditor |
| performing-aws-privilege-escalation-assessment | AWS boto3 SDK |
| detecting-insider-threat-with-ueba | Elasticsearch |
| detecting-malicious-npm-packages | GuardDog/Semgrep |
| detecting-misconfigured-azure-storage | Azure CLI |
| detecting-network-anomalies-with-zeek | Zeek (nids) |
| detecting-port-scanning-with-fail2ban | Fail2ban |
| detecting-privilege-escalation-in-kubernetes-pods | kubectl |
| detecting-process-injection-techniques | Volatility |

## PROSE (136)

- exploiting-sql-injection-vulnerabilities
- exploiting-websocket-vulnerabilities
- generating-threat-intelligence-reports
- hunting-advanced-persistent-threats
- implementing-aes-encryption-for-data-at-rest
- eradicating-malware-from-infected-systems
- evaluating-threat-intelligence-platforms
- executing-nist-rmf-authorization-to-operate
- building-attack-pattern-library-from-cti-reports
- building-c2-infrastructure-with-sliver-framework
- building-c2-redirector-infrastructure
- building-cloud-siem-with-sentinel
- building-detection-rule-with-splunk-spl
- building-detection-rules-with-sigma
- building-devsecops-pipeline-with-gitlab-ci
- building-identity-federation-with-saml-azure-ad
- building-identity-governance-lifecycle-process
- building-incident-response-dashboard
- building-incident-response-playbook
- building-incident-timeline-with-timesketch
- building-ioc-enrichment-pipeline-with-opencti
- building-malware-incident-communication-template
- building-patch-tuesday-response-process
- building-phishing-reporting-button-workflow
- building-ransomware-playbook-with-cisa-framework
- building-red-team-c2-infrastructure-with-havoc
- building-soc-escalation-matrix
- conducting-post-incident-lessons-learned
- conducting-social-engineering-penetration-test
- conducting-social-engineering-pretext-call
- conducting-spearphishing-simulation-campaign
- configuring-aws-verified-access-for-ztna
- configuring-certificate-authority-with-openssl
- configuring-hsm-for-key-storage
- configuring-identity-aware-proxy-with-google-iap
- configuring-microsegmentation-for-zero-trust
- configuring-multi-factor-authentication-with-duo
- configuring-network-segmentation-with-vlans
- configuring-oauth2-authorization-flow
- configuring-snort-ids-for-intrusion-detection
- configuring-suricata-for-network-monitoring
- configuring-tls-1-3-for-secure-communications
- configuring-windows-defender-advanced-settings
- configuring-windows-event-logging-for-detection
- configuring-zscaler-private-access-for-ztna
- containing-active-breach
- continuous-llm-red-teaming-with-promptfoo
- building-soc-metrics-and-kpi-tracking
- building-soc-playbook-for-ransomware
- building-threat-actor-profile-from-osint
- building-threat-feed-aggregation-with-misp
- building-threat-hunt-hypothesis-framework
- building-threat-intelligence-enrichment-in-splunk
- building-threat-intelligence-feed-integration
- building-threat-intelligence-platform
- building-vulnerability-aging-and-sla-tracking
- building-vulnerability-dashboard-with-defectdojo
- building-vulnerability-exception-tracking-system
- collecting-threat-intelligence-with-misp
- collecting-volatile-evidence-from-compromised-host
- conducting-cloud-penetration-testing
- conducting-cyber-risk-assessment-with-nist-800-30
- conducting-external-reconnaissance-with-osint
- conducting-full-scope-red-team-engagement
- executing-red-team-engagement-planning
- executing-red-team-exercise
- exploiting-bgp-hijacking-vulnerabilities
- exploiting-ipv6-vulnerabilities
- implementing-gdpr-data-protection-controls
- implementing-hardware-security-key-authentication
- implementing-honeypot-for-ransomware-detection
- implementing-conduit-security-for-ot-remote-access
- implementing-diamond-model-analysis
- implementing-purdue-model-network-segmentation
- implementing-ransomware-backup-strategy
- implementing-security-chaos-engineering
- implementing-zero-standing-privilege-with-cyberark
- implementing-zero-trust-dns-with-nextdns
- implementing-zero-trust-for-saas-applications
- implementing-zero-trust-in-cloud
- implementing-zero-trust-network-access-with-zscaler
- implementing-zero-trust-network-access
- implementing-zero-trust-with-beyondcorp
- implementing-zero-trust-with-hashicorp-boundary
- intercepting-mobile-traffic-with-burpsuite
- managing-cloud-identity-with-okta
- managing-intelligence-lifecycle
- managing-third-party-vendor-risk
- mapping-mitre-attack-techniques
- correlating-threat-campaigns
- deobfuscating-powershell-obfuscated-malware
- deploying-active-directory-honeytokens
- deploying-cloud-deception-with-decoy-resources
- deploying-cloudflare-access-for-zero-trust
- deploying-decoy-files-for-ransomware-detection
- deploying-honeytokens-and-canarytokens
- deploying-osquery-for-endpoint-monitoring
- deploying-palo-alto-prisma-access-zero-trust
- deploying-ransomware-canary-files
- deploying-software-defined-perimeter
- deploying-tailscale-for-zero-trust-vpn
- designing-adversary-engagement-with-mitre-engage
- detecting-attacks-on-historian-servers
- detecting-attacks-on-scada-systems
- achieving-cmmc-level-2-compliance
- acquiring-disk-image-with-dd-and-dcfldd
- analyzing-campaign-attribution-evidence
- analyzing-cyber-kill-chain
- implementing-anti-phishing-training-program
- implementing-cisa-zero-trust-maturity-model
- triaging-security-incident-with-ir-playbook
- triaging-security-incident
- performing-threat-landscape-assessment-for-sector
- performing-threat-modeling-with-owasp-threat-dragon
- recovering-from-ransomware-attack
- implementing-mitre-attack-coverage-mapping
- implementing-nerc-cip-compliance-controls
- implementing-network-deception-with-honeypots
- implementing-ot-incident-response-playbook
- performing-phishing-simulation-with-gophish
- performing-physical-intrusion-assessment
- performing-power-grid-cybersecurity-assessment
- performing-privacy-impact-assessment
- performing-purple-team-atomic-testing
- performing-purple-team-exercise
- performing-ransomware-response
- performing-ransomware-tabletop-exercise
- performing-soc-tabletop-exercise
- performing-soc2-type2-audit-preparation
- performing-nist-csf-maturity-assessment
- securing-remote-access-to-ot-environment
- testing-ransomware-recovery-procedures
- operating-sliver-c2
- performing-access-review-and-certification
- performing-active-directory-forest-trust-attack
- detecting-lateral-movement-with-splunk

