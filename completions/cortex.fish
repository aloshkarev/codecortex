# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_cortex_global_optspecs
	string join \n format= v/verbose h/help V/version
end

function __fish_cortex_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_cortex_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_cortex_using_subcommand
	set -l cmd (__fish_cortex_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c cortex -n "__fish_cortex_needs_command" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_needs_command" -s v -l verbose
complete -c cortex -n "__fish_cortex_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_needs_command" -s V -l version -d 'Print version'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "setup"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "doctor"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "mcp"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "index"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "watch"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "unwatch"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "find"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "analyze"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "bundle"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "config"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "clean"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "list"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "delete"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "stats"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "query"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "jobs"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "debug"
complete -c cortex -n "__fish_cortex_needs_command" -f -a "completion" -d 'Generate shell completion scripts'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "interactive" -d 'Start interactive REPL mode'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "capsule" -d 'Get context capsule for a symbol'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "impact" -d 'Get impact graph for a symbol'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "refactor" -d 'Analyze refactoring suggestions for a symbol'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "patterns" -d 'Find design patterns in codebase'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "test" -d 'Find tests for a symbol'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "diagnose" -d 'Run diagnostic checks'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "memory" -d 'Memory operations'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "project" -d 'Project management operations'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "skeleton" -d 'Get skeleton (compressed view) of a file'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "signature" -d 'Get signature of a symbol'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "search" -d 'Semantic code search using vector embeddings'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "vector-index" -d 'Index code for vector search'
complete -c cortex -n "__fish_cortex_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand setup" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand setup" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand setup" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand doctor" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand doctor" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand doctor" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -f -a "start"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -f -a "tools"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from start" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from start" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from start" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from tools" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from tools" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from tools" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from help" -f -a "start"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from help" -f -a "tools"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand index" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand index" -l force
complete -c cortex -n "__fish_cortex_using_subcommand index" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand index" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand watch" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand watch" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand watch" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand unwatch" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand unwatch" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand unwatch" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "name"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "pattern"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "type"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "content"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "decorator"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "argument"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from name" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from name" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from name" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from pattern" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from pattern" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from pattern" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from type" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from type" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from type" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from content" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from content" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from content" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from decorator" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from decorator" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from decorator" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from argument" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from argument" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from argument" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "name"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "pattern"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "type"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "content"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "decorator"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "argument"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "callers"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "callees"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "chain"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "hierarchy"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "deps"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "dead-code"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "complexity"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "overrides"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callers" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callers" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callers" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callees" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callees" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callees" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from chain" -l depth -r
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from chain" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from chain" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from chain" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from hierarchy" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from hierarchy" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from hierarchy" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from deps" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from deps" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from deps" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from dead-code" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from dead-code" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from dead-code" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from complexity" -l top -r
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from complexity" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from complexity" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from complexity" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from overrides" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from overrides" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from overrides" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "callers"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "callees"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "chain"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "hierarchy"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "deps"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "dead-code"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "complexity"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "overrides"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -f -a "export"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -f -a "import"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from export" -l repo -r -F
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from export" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from export" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from export" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from import" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from import" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from import" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from help" -f -a "export"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from help" -f -a "import"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -f -a "show"
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -f -a "set"
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -f -a "reset"
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from show" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from show" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from set" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from set" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from reset" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from reset" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from reset" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "show"
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "set"
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "reset"
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand clean" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand clean" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand clean" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand list" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand list" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand delete" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand delete" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand delete" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand stats" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand stats" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand stats" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand query" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand query" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand query" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -f -a "list"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -f -a "status"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from list" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from list" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from status" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from status" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from help" -f -a "list"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from help" -f -a "status"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "capsule" -d 'Debug context capsule building for a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "cache" -d 'Show cache statistics'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "trace" -d 'Trace query execution'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "validate" -d 'Validate index integrity'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -l max-items -d 'Maximum items in capsule' -r
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -l explain -d 'Explain the capsule building process'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from cache" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from cache" -l clear -d 'Clear the cache'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from cache" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from cache" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from trace" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from trace" -s v -l verbose -d 'Enable verbose output'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from trace" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -l repo -d 'Repository path to validate' -r
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -l fix -d 'Fix issues automatically'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "capsule" -d 'Debug context capsule building for a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "cache" -d 'Show cache statistics'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "trace" -d 'Trace query execution'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "validate" -d 'Validate index integrity'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand completion" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand completion" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand completion" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand interactive" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand interactive" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand interactive" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand capsule" -l max-items -d 'Maximum items in capsule' -r
complete -c cortex -n "__fish_cortex_using_subcommand capsule" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand capsule" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand capsule" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand impact" -l depth -d 'Maximum depth to traverse' -r
complete -c cortex -n "__fish_cortex_using_subcommand impact" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand impact" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand impact" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand refactor" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand refactor" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand refactor" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand patterns" -l pattern-type -d 'Filter by pattern type (singleton, factory, observer, etc.)' -r
complete -c cortex -n "__fish_cortex_using_subcommand patterns" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand patterns" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand patterns" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand test" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand test" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand test" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand diagnose" -l component -d 'Check specific component' -r
complete -c cortex -n "__fish_cortex_using_subcommand diagnose" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand diagnose" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand diagnose" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and not __fish_seen_subcommand_from save search context list clear help" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand memory; and not __fish_seen_subcommand_from save search context list clear help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand memory; and not __fish_seen_subcommand_from save search context list clear help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and not __fish_seen_subcommand_from save search context list clear help" -f -a "save" -d 'Save an observation'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and not __fish_seen_subcommand_from save search context list clear help" -f -a "search" -d 'Search observations'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and not __fish_seen_subcommand_from save search context list clear help" -f -a "context" -d 'Get session context'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and not __fish_seen_subcommand_from save search context list clear help" -f -a "list" -d 'List all observations'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and not __fish_seen_subcommand_from save search context list clear help" -f -a "clear" -d 'Clear all observations'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and not __fish_seen_subcommand_from save search context list clear help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from save" -l classification -d 'Classification (architecture, decision, pattern, issue, note)' -r
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from save" -l severity -d 'Severity (low, medium, high, critical)' -r
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from save" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from save" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from save" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from search" -l limit -d 'Maximum results' -r
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from search" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from search" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from search" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from context" -l session -d 'Session ID (optional)' -r
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from context" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from context" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from context" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from list" -l classification -d 'Filter by classification' -r
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from list" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from list" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from clear" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from clear" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from clear" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from help" -f -a "save" -d 'Save an observation'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from help" -f -a "search" -d 'Search observations'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from help" -f -a "context" -d 'Get session context'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from help" -f -a "list" -d 'List all observations'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from help" -f -a "clear" -d 'Clear all observations'
complete -c cortex -n "__fish_cortex_using_subcommand memory; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -f -a "list" -d 'List all registered projects'
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -f -a "add" -d 'Add a project to the registry'
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -f -a "remove" -d 'Remove a project from the registry'
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -f -a "set" -d 'Set the current active project'
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -f -a "current" -d 'Get the current active project'
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -f -a "branches" -d 'List branches for a project'
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -f -a "refresh" -d 'Refresh Git info for a project'
complete -c cortex -n "__fish_cortex_using_subcommand project; and not __fish_seen_subcommand_from list add remove set current branches refresh help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from list" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from list" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from add" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from add" -l track-branch -d 'Whether to track branch changes'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from add" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from remove" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from remove" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from set" -l branch -d 'Branch to use (optional, defaults to current)' -r
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from set" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from set" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from current" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from current" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from current" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from branches" -l path -d 'Path to the project (optional, uses current)' -r -F
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from branches" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from branches" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from branches" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from refresh" -l path -d 'Path to the project (optional, uses current)' -r -F
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from refresh" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from refresh" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from refresh" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from help" -f -a "list" -d 'List all registered projects'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add a project to the registry'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove a project from the registry'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set the current active project'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from help" -f -a "current" -d 'Get the current active project'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from help" -f -a "branches" -d 'List branches for a project'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from help" -f -a "refresh" -d 'Refresh Git info for a project'
complete -c cortex -n "__fish_cortex_using_subcommand project; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand skeleton" -l mode -d 'Skeleton mode (minimal, standard, full)' -r
complete -c cortex -n "__fish_cortex_using_subcommand skeleton" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand skeleton" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand skeleton" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand signature" -l repo -d 'Repository path filter' -r
complete -c cortex -n "__fish_cortex_using_subcommand signature" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand signature" -l include-related -d 'Include related symbols'
complete -c cortex -n "__fish_cortex_using_subcommand signature" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand signature" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand search" -l limit -d 'Number of results to return' -r
complete -c cortex -n "__fish_cortex_using_subcommand search" -l search-type -d 'Search type (semantic, structural, hybrid)' -r
complete -c cortex -n "__fish_cortex_using_subcommand search" -l repo -d 'Filter by repository path' -r
complete -c cortex -n "__fish_cortex_using_subcommand search" -l path -d 'Filter by file path pattern' -r
complete -c cortex -n "__fish_cortex_using_subcommand search" -l kind -d 'Filter by symbol kind (function, class, method, etc.)' -r
complete -c cortex -n "__fish_cortex_using_subcommand search" -l language -d 'Filter by language' -r
complete -c cortex -n "__fish_cortex_using_subcommand search" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand search" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand search" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand vector-index" -l repo -d 'Repository path for metadata' -r
complete -c cortex -n "__fish_cortex_using_subcommand vector-index" -l format -d 'Output format (format, json-pretty, yaml, table)' -r -f -a "json\t'JSON output (default)'
json-pretty\t'Pretty JSON with indentation'
yaml\t'YAML output'
table\t'Table format for tabular data'"
complete -c cortex -n "__fish_cortex_using_subcommand vector-index" -l force -d 'Force reindex'
complete -c cortex -n "__fish_cortex_using_subcommand vector-index" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand vector-index" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "setup"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "doctor"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "mcp"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "index"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "watch"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "unwatch"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "find"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "analyze"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "bundle"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "config"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "clean"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "list"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "delete"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "stats"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "query"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "jobs"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "debug"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "completion" -d 'Generate shell completion scripts'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "interactive" -d 'Start interactive REPL mode'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "capsule" -d 'Get context capsule for a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "impact" -d 'Get impact graph for a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "refactor" -d 'Analyze refactoring suggestions for a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "patterns" -d 'Find design patterns in codebase'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "test" -d 'Find tests for a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "diagnose" -d 'Run diagnostic checks'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "memory" -d 'Memory operations'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "project" -d 'Project management operations'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "skeleton" -d 'Get skeleton (compressed view) of a file'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "signature" -d 'Get signature of a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "search" -d 'Semantic code search using vector embeddings'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "vector-index" -d 'Index code for vector search'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive capsule impact refactor patterns test diagnose memory project skeleton signature search vector-index help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from mcp" -f -a "start"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from mcp" -f -a "tools"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from find" -f -a "name"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from find" -f -a "pattern"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from find" -f -a "type"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from find" -f -a "content"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from find" -f -a "decorator"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from find" -f -a "argument"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from analyze" -f -a "callers"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from analyze" -f -a "callees"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from analyze" -f -a "chain"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from analyze" -f -a "hierarchy"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from analyze" -f -a "deps"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from analyze" -f -a "dead-code"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from analyze" -f -a "complexity"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from analyze" -f -a "overrides"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from bundle" -f -a "export"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from bundle" -f -a "import"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "show"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "set"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "reset"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from jobs" -f -a "list"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from jobs" -f -a "status"
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from debug" -f -a "capsule" -d 'Debug context capsule building for a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from debug" -f -a "cache" -d 'Show cache statistics'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from debug" -f -a "trace" -d 'Trace query execution'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from debug" -f -a "validate" -d 'Validate index integrity'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from memory" -f -a "save" -d 'Save an observation'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from memory" -f -a "search" -d 'Search observations'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from memory" -f -a "context" -d 'Get session context'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from memory" -f -a "list" -d 'List all observations'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from memory" -f -a "clear" -d 'Clear all observations'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from project" -f -a "list" -d 'List all registered projects'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from project" -f -a "add" -d 'Add a project to the registry'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from project" -f -a "remove" -d 'Remove a project from the registry'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from project" -f -a "set" -d 'Set the current active project'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from project" -f -a "current" -d 'Get the current active project'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from project" -f -a "branches" -d 'List branches for a project'
complete -c cortex -n "__fish_cortex_using_subcommand help; and __fish_seen_subcommand_from project" -f -a "refresh" -d 'Refresh Git info for a project'
