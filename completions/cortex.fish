# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_cortex_global_optspecs
	string join \n json v/verbose h/help V/version
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

complete -c cortex -n "__fish_cortex_needs_command" -l json
complete -c cortex -n "__fish_cortex_needs_command" -s v -l verbose
complete -c cortex -n "__fish_cortex_needs_command" -s h -l help -d 'Print help'
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
complete -c cortex -n "__fish_cortex_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand setup" -l json
complete -c cortex -n "__fish_cortex_using_subcommand setup" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand setup" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand doctor" -l json
complete -c cortex -n "__fish_cortex_using_subcommand doctor" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -l json
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -f -a "start"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -f -a "tools"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and not __fish_seen_subcommand_from start tools help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from start" -l json
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from start" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from start" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from tools" -l json
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from tools" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from tools" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from help" -f -a "start"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from help" -f -a "tools"
complete -c cortex -n "__fish_cortex_using_subcommand mcp; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand index" -l force
complete -c cortex -n "__fish_cortex_using_subcommand index" -l json
complete -c cortex -n "__fish_cortex_using_subcommand index" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand index" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand watch" -l json
complete -c cortex -n "__fish_cortex_using_subcommand watch" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand watch" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand unwatch" -l json
complete -c cortex -n "__fish_cortex_using_subcommand unwatch" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand unwatch" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -l json
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "name"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "pattern"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "type"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "content"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "decorator"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "argument"
complete -c cortex -n "__fish_cortex_using_subcommand find; and not __fish_seen_subcommand_from name pattern type content decorator argument help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from name" -l json
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from name" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from name" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from pattern" -l json
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from pattern" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from pattern" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from type" -l json
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from type" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from type" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from content" -l json
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from content" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from content" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from decorator" -l json
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from decorator" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from decorator" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from argument" -l json
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from argument" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from argument" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "name"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "pattern"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "type"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "content"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "decorator"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "argument"
complete -c cortex -n "__fish_cortex_using_subcommand find; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -l json
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "callers"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "callees"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "chain"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "hierarchy"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "deps"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "dead-code"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "complexity"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "overrides"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and not __fish_seen_subcommand_from callers callees chain hierarchy deps dead-code complexity overrides help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callers" -l json
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callers" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callers" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callees" -l json
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callees" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from callees" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from chain" -l depth -r
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from chain" -l json
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from chain" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from chain" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from hierarchy" -l json
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from hierarchy" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from hierarchy" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from deps" -l json
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from deps" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from deps" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from dead-code" -l json
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from dead-code" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from dead-code" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from complexity" -l top -r
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from complexity" -l json
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from complexity" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from complexity" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from overrides" -l json
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from overrides" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from overrides" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "callers"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "callees"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "chain"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "hierarchy"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "deps"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "dead-code"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "complexity"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "overrides"
complete -c cortex -n "__fish_cortex_using_subcommand analyze; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -l json
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -f -a "export"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -f -a "import"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and not __fish_seen_subcommand_from export import help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from export" -l repo -r -F
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from export" -l json
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from export" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from export" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from import" -l json
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from import" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from import" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from help" -f -a "export"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from help" -f -a "import"
complete -c cortex -n "__fish_cortex_using_subcommand bundle; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -l json
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -f -a "show"
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -f -a "set"
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -f -a "reset"
complete -c cortex -n "__fish_cortex_using_subcommand config; and not __fish_seen_subcommand_from show set reset help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from show" -l json
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from show" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from set" -l json
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from set" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from reset" -l json
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from reset" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from reset" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "show"
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "set"
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "reset"
complete -c cortex -n "__fish_cortex_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand clean" -l json
complete -c cortex -n "__fish_cortex_using_subcommand clean" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand clean" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand list" -l json
complete -c cortex -n "__fish_cortex_using_subcommand list" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand list" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand delete" -l json
complete -c cortex -n "__fish_cortex_using_subcommand delete" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand delete" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand stats" -l json
complete -c cortex -n "__fish_cortex_using_subcommand stats" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand stats" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand query" -l json
complete -c cortex -n "__fish_cortex_using_subcommand query" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand query" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -l json
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -f -a "list"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -f -a "status"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and not __fish_seen_subcommand_from list status help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from list" -l json
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from list" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from status" -l json
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from status" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from help" -f -a "list"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from help" -f -a "status"
complete -c cortex -n "__fish_cortex_using_subcommand jobs; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -l json
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "capsule" -d 'Debug context capsule building for a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "cache" -d 'Show cache statistics'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "trace" -d 'Trace query execution'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "validate" -d 'Validate index integrity'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and not __fish_seen_subcommand_from capsule cache trace validate help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -l max-items -d 'Maximum items in capsule' -r
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -l explain -d 'Explain the capsule building process'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -l json
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from capsule" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from cache" -l clear -d 'Clear the cache'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from cache" -l json
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from cache" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from cache" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from trace" -s v -l verbose -d 'Enable verbose output'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from trace" -l json
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from trace" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -l repo -d 'Repository path to validate' -r
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -l fix -d 'Fix issues automatically'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -l json
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from validate" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "capsule" -d 'Debug context capsule building for a symbol'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "cache" -d 'Show cache statistics'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "trace" -d 'Trace query execution'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "validate" -d 'Validate index integrity'
complete -c cortex -n "__fish_cortex_using_subcommand debug; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cortex -n "__fish_cortex_using_subcommand completion" -l json
complete -c cortex -n "__fish_cortex_using_subcommand completion" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand completion" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand interactive" -l json
complete -c cortex -n "__fish_cortex_using_subcommand interactive" -s v -l verbose
complete -c cortex -n "__fish_cortex_using_subcommand interactive" -s h -l help -d 'Print help'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "setup"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "doctor"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "mcp"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "index"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "watch"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "unwatch"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "find"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "analyze"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "bundle"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "config"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "clean"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "list"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "delete"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "stats"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "query"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "jobs"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "debug"
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "completion" -d 'Generate shell completion scripts'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "interactive" -d 'Start interactive REPL mode'
complete -c cortex -n "__fish_cortex_using_subcommand help; and not __fish_seen_subcommand_from setup doctor mcp index watch unwatch find analyze bundle config clean list delete stats query jobs debug completion interactive help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
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
