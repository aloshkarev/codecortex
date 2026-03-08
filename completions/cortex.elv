
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
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
            cand setup 'setup'
            cand doctor 'doctor'
            cand daemon 'daemon'
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
            cand capsule 'Get context capsule for a symbol'
            cand impact 'Get impact graph for a symbol'
            cand refactor 'Analyze refactoring suggestions for a symbol'
            cand patterns 'Find design patterns in codebase'
            cand test 'Find tests for a symbol'
            cand diagnose 'Run diagnostic checks'
            cand memory 'Memory operations'
            cand project 'Project management operations'
            cand skeleton 'Get skeleton (compressed view) of a file'
            cand signature 'Get signature of a symbol'
            cand search 'Semantic code search using vector embeddings'
            cand vector-index 'Index code for vector search'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;setup'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;doctor'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;daemon'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand start 'Start daemon in background'
            cand stop 'Stop daemon process'
            cand status 'Show daemon runtime status'
            cand run 'Run daemon foreground loop (internal)'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;daemon;start'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;daemon;stop'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;daemon;status'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;daemon;run'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;daemon;help'= {
            cand start 'Start daemon in background'
            cand stop 'Stop daemon process'
            cand status 'Show daemon runtime status'
            cand run 'Run daemon foreground loop (internal)'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;daemon;help;start'= {
        }
        &'cortex;daemon;help;stop'= {
        }
        &'cortex;daemon;help;status'= {
        }
        &'cortex;daemon;help;run'= {
        }
        &'cortex;daemon;help;help'= {
        }
        &'cortex;mcp'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand start 'start'
            cand tools 'tools'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;mcp;start'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;mcp;tools'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
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
            cand --mode 'Indexing mode'
            cand --base-branch 'Base branch to use for incremental-diff mode'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand --force 'force'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;watch'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;unwatch'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;find'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand name 'name'
            cand pattern 'pattern'
            cand type 'type'
            cand content 'content'
            cand decorator 'decorator'
            cand argument 'argument'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;find;name'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;find;pattern'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;find;type'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;find;content'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;find;decorator'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;find;argument'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
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
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand callers 'callers'
            cand callees 'callees'
            cand chain 'chain'
            cand hierarchy 'hierarchy'
            cand deps 'deps'
            cand dead-code 'dead-code'
            cand complexity 'complexity'
            cand overrides 'overrides'
            cand smells 'Detect code smells from source files'
            cand refactoring 'Recommend refactoring techniques based on detected smells'
            cand branch-diff 'Compare two git branches for a project/repository'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;analyze;callers'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;callees'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;chain'= {
            cand --depth 'depth'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;hierarchy'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;deps'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;dead-code'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;complexity'= {
            cand --top 'top'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;overrides'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;smells'= {
            cand --min-severity 'Minimum severity to report (info, warning, error, critical)'
            cand --max-files 'Maximum number of files to scan'
            cand --limit 'Maximum number of findings to return'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;refactoring'= {
            cand --min-severity 'Minimum smell severity to consider (info, warning, error, critical)'
            cand --max-files 'Maximum number of files to scan'
            cand --limit 'Maximum number of recommendations to return'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;analyze;branch-diff'= {
            cand --path 'Repository path (optional, uses current project or cwd)'
            cand --commit-limit 'Maximum number of ahead/behind commits returned per side'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
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
            cand smells 'Detect code smells from source files'
            cand refactoring 'Recommend refactoring techniques based on detected smells'
            cand branch-diff 'Compare two git branches for a project/repository'
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
        &'cortex;analyze;help;smells'= {
        }
        &'cortex;analyze;help;refactoring'= {
        }
        &'cortex;analyze;help;branch-diff'= {
        }
        &'cortex;analyze;help;help'= {
        }
        &'cortex;bundle'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand export 'export'
            cand import 'import'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;bundle;export'= {
            cand --repo 'repo'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;bundle;import'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
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
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand show 'show'
            cand set 'set'
            cand reset 'reset'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;config;show'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;config;set'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;config;reset'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
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
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;list'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;delete'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;stats'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;query'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;jobs'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand list 'list'
            cand status 'status'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;jobs;list'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;jobs;status'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
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
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand capsule 'Debug context capsule building for a symbol'
            cand cache 'Show cache statistics'
            cand trace 'Trace query execution'
            cand validate 'Validate index integrity'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;debug;capsule'= {
            cand --max-items 'Maximum items in capsule'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand --explain 'Explain the capsule building process'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;debug;cache'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand --clear 'Clear the cache'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;debug;trace'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'Enable verbose output'
            cand --verbose 'Enable verbose output'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;debug;validate'= {
            cand --repo 'Repository path to validate'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand --fix 'Fix issues automatically'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
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
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;interactive'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;capsule'= {
            cand --max-items 'Maximum items in capsule'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;impact'= {
            cand --depth 'Maximum depth to traverse'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;refactor'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;patterns'= {
            cand --pattern-type 'Filter by pattern type (singleton, factory, observer, etc.)'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;test'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;diagnose'= {
            cand --component 'Check specific component'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;memory'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand save 'Save an observation'
            cand search 'Search observations'
            cand context 'Get session context'
            cand list 'List all observations'
            cand clear 'Clear all observations'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;memory;save'= {
            cand --classification 'Classification (architecture, decision, pattern, issue, note)'
            cand --severity 'Severity (low, medium, high, critical)'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;memory;search'= {
            cand --limit 'Maximum results'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;memory;context'= {
            cand --session 'Session ID (optional)'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;memory;list'= {
            cand --classification 'Filter by classification'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;memory;clear'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;memory;help'= {
            cand save 'Save an observation'
            cand search 'Search observations'
            cand context 'Get session context'
            cand list 'List all observations'
            cand clear 'Clear all observations'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;memory;help;save'= {
        }
        &'cortex;memory;help;search'= {
        }
        &'cortex;memory;help;context'= {
        }
        &'cortex;memory;help;list'= {
        }
        &'cortex;memory;help;clear'= {
        }
        &'cortex;memory;help;help'= {
        }
        &'cortex;project'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand list 'List all registered projects'
            cand add 'Add a project to the registry'
            cand remove 'Remove a project from the registry'
            cand set 'Set the current active project'
            cand current 'Get the current active project'
            cand branches 'List branches for a project'
            cand refresh 'Refresh Git info for a project'
            cand status 'Show project indexing freshness/health status'
            cand sync 'Sync project state: refresh -> detect switch -> index/queue -> cleanup'
            cand policy 'Project branch/indexing policy controls'
            cand metrics 'Show daemon/project metrics snapshot'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;project;list'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;add'= {
            cand --track-branch 'Whether to track branch changes'
            cand --auto-index 'Automatically index checked-out branch after adding'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;remove'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;set'= {
            cand --branch 'Branch to use (optional, defaults to current)'
            cand --auto-index 'Automatically index checked-out branch after switching context'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;current'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;branches'= {
            cand --path 'Path to the project (optional, uses current)'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;refresh'= {
            cand --path 'Path to the project (optional, uses current)'
            cand --auto-index 'Automatically index when a branch switch is detected'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;status'= {
            cand --path 'Path to the project (optional, uses current)'
            cand --include-queue 'Include daemon queue details for this project'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;sync'= {
            cand --path 'Path to the project (optional, uses current)'
            cand --cleanup-old-branches 'Cleanup old branch indexes after sync'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand --force 'Force full indexing when syncing'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;policy'= {
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand show 'Show current project policy'
            cand set 'Update project policy fields'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;project;policy;show'= {
            cand --path 'Path to the project (optional, uses current)'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;policy;set'= {
            cand --path 'Path to the project (optional, uses current)'
            cand --index-only 'Branch allowlist for indexing (repeatable). Empty keeps current value'
            cand --exclude-pattern 'Exclude patterns for indexing (repeatable)'
            cand --max-parallel-index-jobs 'Maximum parallel daemon index jobs for this project'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;policy;help'= {
            cand show 'Show current project policy'
            cand set 'Update project policy fields'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;project;policy;help;show'= {
        }
        &'cortex;project;policy;help;set'= {
        }
        &'cortex;project;policy;help;help'= {
        }
        &'cortex;project;metrics'= {
            cand --path 'Path to the project (optional, uses current)'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;project;help'= {
            cand list 'List all registered projects'
            cand add 'Add a project to the registry'
            cand remove 'Remove a project from the registry'
            cand set 'Set the current active project'
            cand current 'Get the current active project'
            cand branches 'List branches for a project'
            cand refresh 'Refresh Git info for a project'
            cand status 'Show project indexing freshness/health status'
            cand sync 'Sync project state: refresh -> detect switch -> index/queue -> cleanup'
            cand policy 'Project branch/indexing policy controls'
            cand metrics 'Show daemon/project metrics snapshot'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;project;help;list'= {
        }
        &'cortex;project;help;add'= {
        }
        &'cortex;project;help;remove'= {
        }
        &'cortex;project;help;set'= {
        }
        &'cortex;project;help;current'= {
        }
        &'cortex;project;help;branches'= {
        }
        &'cortex;project;help;refresh'= {
        }
        &'cortex;project;help;status'= {
        }
        &'cortex;project;help;sync'= {
        }
        &'cortex;project;help;policy'= {
            cand show 'Show current project policy'
            cand set 'Update project policy fields'
        }
        &'cortex;project;help;policy;show'= {
        }
        &'cortex;project;help;policy;set'= {
        }
        &'cortex;project;help;metrics'= {
        }
        &'cortex;project;help;help'= {
        }
        &'cortex;skeleton'= {
            cand --mode 'Skeleton mode (minimal, standard, full)'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;signature'= {
            cand --repo 'Repository path filter'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand --include-related 'Include related symbols'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;search'= {
            cand --limit 'Number of results to return'
            cand --search-type 'Search type (semantic, structural, hybrid)'
            cand --repo 'Filter by repository path'
            cand --path 'Filter by file path pattern'
            cand --kind 'Filter by symbol kind (function, class, method, etc.)'
            cand --language 'Filter by language'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;vector-index'= {
            cand --repo 'Repository path for metadata'
            cand --format 'Output format (format, json-pretty, yaml, table)'
            cand --force 'Force reindex'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'cortex;help'= {
            cand setup 'setup'
            cand doctor 'doctor'
            cand daemon 'daemon'
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
            cand capsule 'Get context capsule for a symbol'
            cand impact 'Get impact graph for a symbol'
            cand refactor 'Analyze refactoring suggestions for a symbol'
            cand patterns 'Find design patterns in codebase'
            cand test 'Find tests for a symbol'
            cand diagnose 'Run diagnostic checks'
            cand memory 'Memory operations'
            cand project 'Project management operations'
            cand skeleton 'Get skeleton (compressed view) of a file'
            cand signature 'Get signature of a symbol'
            cand search 'Semantic code search using vector embeddings'
            cand vector-index 'Index code for vector search'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'cortex;help;setup'= {
        }
        &'cortex;help;doctor'= {
        }
        &'cortex;help;daemon'= {
            cand start 'Start daemon in background'
            cand stop 'Stop daemon process'
            cand status 'Show daemon runtime status'
            cand run 'Run daemon foreground loop (internal)'
        }
        &'cortex;help;daemon;start'= {
        }
        &'cortex;help;daemon;stop'= {
        }
        &'cortex;help;daemon;status'= {
        }
        &'cortex;help;daemon;run'= {
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
            cand smells 'Detect code smells from source files'
            cand refactoring 'Recommend refactoring techniques based on detected smells'
            cand branch-diff 'Compare two git branches for a project/repository'
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
        &'cortex;help;analyze;smells'= {
        }
        &'cortex;help;analyze;refactoring'= {
        }
        &'cortex;help;analyze;branch-diff'= {
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
        &'cortex;help;capsule'= {
        }
        &'cortex;help;impact'= {
        }
        &'cortex;help;refactor'= {
        }
        &'cortex;help;patterns'= {
        }
        &'cortex;help;test'= {
        }
        &'cortex;help;diagnose'= {
        }
        &'cortex;help;memory'= {
            cand save 'Save an observation'
            cand search 'Search observations'
            cand context 'Get session context'
            cand list 'List all observations'
            cand clear 'Clear all observations'
        }
        &'cortex;help;memory;save'= {
        }
        &'cortex;help;memory;search'= {
        }
        &'cortex;help;memory;context'= {
        }
        &'cortex;help;memory;list'= {
        }
        &'cortex;help;memory;clear'= {
        }
        &'cortex;help;project'= {
            cand list 'List all registered projects'
            cand add 'Add a project to the registry'
            cand remove 'Remove a project from the registry'
            cand set 'Set the current active project'
            cand current 'Get the current active project'
            cand branches 'List branches for a project'
            cand refresh 'Refresh Git info for a project'
            cand status 'Show project indexing freshness/health status'
            cand sync 'Sync project state: refresh -> detect switch -> index/queue -> cleanup'
            cand policy 'Project branch/indexing policy controls'
            cand metrics 'Show daemon/project metrics snapshot'
        }
        &'cortex;help;project;list'= {
        }
        &'cortex;help;project;add'= {
        }
        &'cortex;help;project;remove'= {
        }
        &'cortex;help;project;set'= {
        }
        &'cortex;help;project;current'= {
        }
        &'cortex;help;project;branches'= {
        }
        &'cortex;help;project;refresh'= {
        }
        &'cortex;help;project;status'= {
        }
        &'cortex;help;project;sync'= {
        }
        &'cortex;help;project;policy'= {
            cand show 'Show current project policy'
            cand set 'Update project policy fields'
        }
        &'cortex;help;project;policy;show'= {
        }
        &'cortex;help;project;policy;set'= {
        }
        &'cortex;help;project;metrics'= {
        }
        &'cortex;help;skeleton'= {
        }
        &'cortex;help;signature'= {
        }
        &'cortex;help;search'= {
        }
        &'cortex;help;vector-index'= {
        }
        &'cortex;help;help'= {
        }
    ]
    $completions[$command]
}
