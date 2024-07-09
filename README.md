1. `/runtime`
	- handle execution
	- executes according to a context
	- context determines the execution domain
	- efficiently manages the execution of multiple tenants

2. `/db`
	- database that supports multitenant feature
	- a tenant identified by a tenant id
	- each tenant is isolated from other tenants key-spaces

3. `/context`
	- context is a set of configurations that determine the execution environment
	- manages the creation of a context based on a tenant id

4. `/rpc`
	- receives requests from multiple tenants
	- only responsible for receiving requests and forwarding them to the context manager/runtime
	- rpc layer should be context-agnostic
