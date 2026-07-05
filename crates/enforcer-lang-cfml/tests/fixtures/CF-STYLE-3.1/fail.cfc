<cfcomponent>
    <cffunction name="create" returntype="Order" access="public">
        <cfargument name="id" type="numeric" required="true">
        <cfreturn orderGateway.insert(arguments.id)>
    </cffunction>
</cfcomponent>
