complete -c roco -n "__fish_use_subcommand" -s h -l help -d 'Print help information'
complete -c roco -n "__fish_use_subcommand" -s V -l version -d 'Print version information'
complete -c roco -n "__fish_use_subcommand" -f -a "list"
complete -c roco -n "__fish_use_subcommand" -f -a "bad"
complete -c roco -n "__fish_use_subcommand" -f -a "outdated"
complete -c roco -n "__fish_use_subcommand" -f -a "source"
complete -c roco -n "__fish_use_subcommand" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c roco -n "__fish_seen_subcommand_from list" -s r -l limitoutput -d 'limit the output to essential information'
complete -c roco -n "__fish_seen_subcommand_from list" -s v -l verbose -d 'be verbose'
complete -c roco -n "__fish_seen_subcommand_from list" -s h -l help -d 'Print help information'
complete -c roco -n "__fish_seen_subcommand_from bad" -s r -l limitoutput -d 'limit the output to essential information'
complete -c roco -n "__fish_seen_subcommand_from bad" -s v -l verbose -d 'be verbose'
complete -c roco -n "__fish_seen_subcommand_from bad" -s h -l help -d 'Print help information'
complete -c roco -n "__fish_seen_subcommand_from outdated" -l ignore-pinned -d 'ignore any pinned packages'
complete -c roco -n "__fish_seen_subcommand_from outdated" -l ignore-unfound -d 'ignore any unfound packages'
complete -c roco -n "__fish_seen_subcommand_from outdated" -s p -l pre -d 'include prerelease versions'
complete -c roco -n "__fish_seen_subcommand_from outdated" -s r -l limitoutput -d 'limit the output to essential information'
complete -c roco -n "__fish_seen_subcommand_from outdated" -s v -l verbose -d 'be verbose'
complete -c roco -n "__fish_seen_subcommand_from outdated" -s h -l help -d 'Print help information'
complete -c roco -n "__fish_seen_subcommand_from source" -s r -l limitoutput -d 'limit the output to essential information'
complete -c roco -n "__fish_seen_subcommand_from source" -s v -l verbose -d 'be verbose'
complete -c roco -n "__fish_seen_subcommand_from source" -s h -l help -d 'Print help information'
