<cfcomponent>
<cffunction name="getUser" access="public" returntype="numeric">
    <cfargument name="id" type="numeric">
    <cfset var result = id>
    <cfif result GT 0>
        <cfset logAccess(result)>
    <cfelse>
        <cfset logAccess(0)>
    </cfif>
    <cfreturn result>
</cffunction>
</cfcomponent>
