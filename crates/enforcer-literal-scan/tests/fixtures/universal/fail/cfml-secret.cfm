<!--- Fail fixture for LIT-2.1 (universal literal-scan T2 advisory bridge).
      Dense with hardcoded secret-shaped and literal-risk strings so the
      literal-scan score crosses the advisory threshold. --->
<cfset apiKey = "AKIAABCDEFGHIJKLMNOP">
<cfset authToken = "sk-proj-abcdefghijklmnopqrstuvwxyz123456">
<cfset endpoint = "https://api.internal.example.com/v1/payments">
<cfif status EQ "ready">
    <cfset statusReady = "ready">
</cfif>
