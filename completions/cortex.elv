
use builtin;
use str;

set edit:completion:arg-completer[cortex] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'cortex'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'cortex'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
            cand setup 'setup'
            cand doctor 'doctor'
            cand mcp 'mcp'
            cand index 'index'
            cand watch 'watch'
            cand unwatch 'unwatch'
            cand find 'find'
            cand analyze 'analyze'
            cand bundle 'bundle'
            cand config 'config'
            cand clean 'clean'
            cand list 'list'
            cand delete 'delete'
            cand stats 'stats'
            cand query 'query'
            cand jobs 'jobs'
            cand debug 'debug'
            cand completion 'Generate shell completion scripts'
            cand interactive 'Start interactive REPL mode'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;setup'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;doctor'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;mcp'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand start 'start'
            cand tools 'tools'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;mcp;start'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;mcp;tools'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;mcp;help'= {
            cand start 'start'
            cand tools 'tools'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;mcp;help;start'= {
        }
        &'cortex;mcp;help;tools'= {
        }
        &'cortex;mcp;help;help'= {
        }
        &'cortex;index'= {
            cand --force 'force'
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;watch'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;unwatch'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;find'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand name 'name'
            cand pattern 'pattern'
            cand type 'type'
            cand content 'content'
            cand decorator 'decorator'
            cand argument 'argument'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;find;name'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;find;pattern'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;find;type'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;find;content'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;find;decorator'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;find;argument'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;find;help'= {
            cand name 'name'
            cand pattern 'pattern'
            cand type 'type'
            cand content 'content'
            cand decorator 'decorator'
            cand argument 'argument'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;find;help;name'= {
        }
        &'cortex;find;help;pattern'= {
        }
        &'cortex;find;help;type'= {
        }
        &'cortex;find;help;content'= {
        }
        &'cortex;find;help;decorator'= {
        }
        &'cortex;find;help;argument'= {
        }
        &'cortex;find;help;help'= {
        }
        &'cortex;analyze'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand callers 'callers'
            cand callees 'callees'
            cand chain 'chain'
            cand hierarchy 'hierarchy'
            cand deps 'deps'
            cand dead-code 'dead-code'
            cand complexity 'complexity'
            cand overrides 'overrides'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;analyze;callers'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;analyze;callees'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;analyze;chain'= {
            cand --depth 'depth'
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;analyze;hierarchy'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;analyze;deps'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;analyze;dead-code'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;analyze;complexity'= {
            cand --top 'top'
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;analyze;overrides'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;analyze;help'= {
            cand callers 'callers'
            cand callees 'callees'
            cand chain 'chain'
            cand hierarchy 'hierarchy'
            cand deps 'deps'
            cand dead-code 'dead-code'
            cand complexity 'complexity'
            cand overrides 'overrides'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;analyze;help;callers'= {
        }
        &'cortex;analyze;help;callees'= {
        }
        &'cortex;analyze;help;chain'= {
        }
        &'cortex;analyze;help;hierarchy'= {
        }
        &'cortex;analyze;help;deps'= {
        }
        &'cortex;analyze;help;dead-code'= {
        }
        &'cortex;analyze;help;complexity'= {
        }
        &'cortex;analyze;help;overrides'= {
        }
        &'cortex;analyze;help;help'= {
        }
        &'cortex;bundle'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand export 'export'
            cand import 'import'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;bundle;export'= {
            cand --repo 'repo'
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;bundle;import'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;bundle;help'= {
            cand export 'export'
            cand import 'import'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;bundle;help;export'= {
        }
        &'cortex;bundle;help;import'= {
        }
        &'cortex;bundle;help;help'= {
        }
        &'cortex;config'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand show 'show'
            cand set 'set'
            cand reset 'reset'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;config;show'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;config;set'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;config;reset'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;config;help'= {
            cand show 'show'
            cand set 'set'
            cand reset 'reset'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;config;help;show'= {
        }
        &'cortex;config;help;set'= {
        }
        &'cortex;config;help;reset'= {
        }
        &'cortex;config;help;help'= {
        }
        &'cortex;clean'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;list'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;delete'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;stats'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;query'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;jobs'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand list 'list'
            cand status 'status'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;jobs;list'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;jobs;status'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;jobs;help'= {
            cand list 'list'
            cand status 'status'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;jobs;help;list'= {
        }
        &'cortex;jobs;help;status'= {
        }
        &'cortex;jobs;help;help'= {
        }
        &'cortex;debug'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand capsule 'Debug context capsule building for a symbol'
            cand cache 'Show cache statistics'
            cand trace 'Trace query execution'
            cand validate 'Validate index integrity'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;debug;capsule'= {
            cand --max-items 'Maximum items in capsule'
            cand --explain 'Explain the capsule building process'
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;debug;cache'= {
            cand --clear 'Clear the cache'
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;debug;trace'= {
            cand -v 'Enable verbose output'
            cand --verbose 'Enable verbose output'
            cand --json 'json'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;debug;validate'= {
            cand --repo 'Repository path to validate'
            cand --fix 'Fix issues automatically'
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;debug;help'= {
            cand capsule 'Debug context capsule building for a symbol'
            cand cache 'Show cache statistics'
            cand trace 'Trace query execution'
            cand validate 'Validate index integrity'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;debug;help;capsule'= {
        }
        &'cortex;debug;help;cache'= {
        }
        &'cortex;debug;help;trace'= {
        }
        &'cortex;debug;help;validate'= {
        }
        &'cortex;debug;help;help'= {
        }
        &'cortex;completion'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;interactive'= {
            cand --json 'json'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'cortex;help'= {
            cand setup 'setup'
            cand doctor 'doctor'
            cand mcp 'mcp'
            cand index 'index'
            cand watch 'watch'
            cand unwatch 'unwatch'
            cand find 'find'
            cand analyze 'analyze'
            cand bundle 'bundle'
            cand config 'config'
            cand clean 'clean'
            cand list 'list'
            cand delete 'delete'
            cand stats 'stats'
            cand query 'query'
            cand jobs 'jobs'
            cand debug 'debug'
            cand completion 'Generate shell completion scripts'
            cand interactive 'Start interactive REPL mode'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;help;setup'= {
        }
        &'cortex;help;doctor'= {
        }
        &'cortex;help;mcp'= {
            cand start 'start'
            cand tools 'tools'
        }
        &'cortex;help;mcp;start'= {
        }
        &'cortex;help;mcp;tools'= {
        }
        &'cortex;help;index'= {
        }
        &'cortex;help;watch'= {
        }
        &'cortex;help;unwatch'= {
        }
        &'cortex;help;find'= {
            cand name 'name'
            cand pattern 'pattern'
            cand type 'type'
            cand content 'content'
            cand decorator 'decorator'
            cand argument 'argument'
        }
        &'cortex;help;find;name'= {
        }
        &'cortex;help;find;pattern'= {
        }
        &'cortex;help;find;type'= {
        }
        &'cortex;help;find;content'= {
        }
        &'cortex;help;find;decorator'= {
        }
        &'cortex;help;find;argument'= {
        }
        &'cortex;help;analyze'= {
            cand callers 'callers'
            cand callees 'callees'
            cand chain 'chain'
            cand hierarchy 'hierarchy'
            cand deps 'deps'
            cand dead-code 'dead-code'
            cand complexity 'complexity'
            cand overrides 'overrides'
        }
        &'cortex;help;analyze;callers'= {
        }
        &'cortex;help;analyze;callees'= {
        }
        &'cortex;help;analyze;chain'= {
        }
        &'cortex;help;analyze;hierarchy'= {
        }
        &'cortex;help;analyze;deps'= {
        }
        &'cortex;help;analyze;dead-code'= {
        }
        &'cortex;help;analyze;complexity'= {
        }
        &'cortex;help;analyze;overrides'= {
        }
        &'cortex;help;bundle'= {
            cand export 'export'
            cand import 'import'
        }
        &'cortex;help;bundle;export'= {
        }
        &'cortex;help;bundle;import'= {
        }
        &'cortex;help;config'= {
            cand show 'show'
            cand set 'set'
            cand reset 'reset'
        }
        &'cortex;help;config;show'= {
        }
        &'cortex;help;config;set'= {
        }
        &'cortex;help;config;reset'= {
        }
        &'cortex;help;clean'= {
        }
        &'cortex;help;list'= {
        }
        &'cortex;help;delete'= {
        }
        &'cortex;help;stats'= {
        }
        &'cortex;help;query'= {
        }
        &'cortex;help;jobs'= {
            cand list 'list'
            cand status 'status'
        }
        &'cortex;help;jobs;list'= {
        }
        &'cortex;help;jobs;status'= {
        }
        &'cortex;help;debug'= {
            cand capsule 'Debug context capsule building for a symbol'
            cand cache 'Show cache statistics'
            cand trace 'Trace query execution'
            cand validate 'Validate index integrity'
        }
        &'cortex;help;debug;capsule'= {
        }
        &'cortex;help;debug;cache'= {
        }
        &'cortex;help;debug;trace'= {
        }
        &'cortex;help;debug;validate'= {
        }
        &'cortex;help;completion'= {
        }
        &'cortex;help;interactive'= {
        }
        &'cortex;help;help'= {
        }
    ]
    $completions[$command]
}
