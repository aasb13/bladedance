project('privirc', 'cpp', 'rust',
  version: '4.10.1',
  default_options: [
    'cpp_std=c++26',
    'rust_std=2024',
    'warning_level=3',
    'buildtype=release',
    'prefix=/home/dr/inspircd/run',
  ]
)

fs = import('fs')

# ── Compiler flags (matches old CORECXXFLAGS) ───────────
add_project_arguments(
  '-fPIC',
  '-fvisibility=hidden',
  '-fvisibility-inlines-hidden',
  '-pipe',
  '-DFMT_SHARED',
  '-Woverloaded-virtual',
  '-Wpedantic',
  '-Wno-format-nonliteral',
  '-Wno-unused-parameter',
  language: 'cpp'
)

# ── Dependencies ────────────────────────────────────────
fmt_dep     = dependency('fmt', required: true)
dl_dep      = dependency('dl', required: false)
pthread_dep = dependency('threads')
mongoc_dep  = dependency('mongoc2', required: false)
sqlite3_dep = dependency('sqlite3', required: false)
argon2_dep   = dependency('libargon2', required: false)
ssl_dep      = dependency('openssl', required: false)
gnutls_dep   = dependency('gnutls', required: false)
pcre2_dep    = dependency('libpcre2-8', required: false)
re2_dep      = dependency('re2', required: false)
ldap_dep     = dependency('openldap', required: false)
maxmind_dep  = dependency('libmaxminddb', required: false)

# ── Include directories ────────────────────────────────
inc = include_directories('include', 'vendor')

# PostgreSQL headers are in a non-standard location
inc_pgsql = include_directories('/usr/include/postgresql')

# ── Socket engine (default: epoll on Linux) ────────────
socket_engine = get_option('socketengine')

# ── Core executable ─────────────────────────────────────
core_src_list = files(
  'src/base.cpp',
  'src/channelmanager_glue.cpp',
  'src/channels.cpp',
  'src/cidr_glue.cpp',
  'src/clientprotocol.cpp',
  'src/commands.cpp',
  'src/configparser.cpp',
  'src/configreader.cpp',
  'src/cull.cpp',
  'src/dynamic.cpp',
  'src/extensible.cpp',
  'src/hashcomp.cpp',
  'src/helperfuncs.cpp',
  'src/inspircd.cpp',
  'src/listensocket.cpp',
  'src/listmode.cpp',
  'src/logging_glue.cpp',
  'src/mode.cpp',
  'src/modulemanager_glue.cpp',
  'src/modules.cpp',
  'src/server.cpp',
  'src/snomasks_glue.cpp',
  'src/socket.cpp',
  'src/socketengine.cpp',
  'src/streamsocket.cpp',
  # 'src/stringutils.cpp',
  'src/thread.cpp',
  'src/timer.cpp',
  'src/usermanager_glue.cpp',
  'src/users_glue.cpp',
  'src/users_ffi.cpp',
  'src/wildcard_glue.cpp',
  'src/xline.cpp',
)

rust_manifest = meson.project_source_root() / 'src' / 'rust' / 'Cargo.toml'
rust_lib_rs = meson.project_source_root() / 'src' / 'rust' / 'lib.rs'
rust_core_rs_files = files(
  'src/rust/Cargo.toml',
  'src/rust/lib.rs',
  'src/rust/Cargo.lock',
  'src/rust_core.rs',
  'src/stringutils.rs',
  'src/bancache.rs',
  'src/usermanager.rs',
  'src/cidr.rs',
  'src/channelmanager.rs',
  'src/snomasks.rs',
  'src/wildcard.rs',
  'src/logging.rs',
  'src/modulemanager.rs',
  'src/users.rs',
  'src/server.rs',
  'src/logging.rs',
  'src/coremods/core_info/cmd_admin.rs',
)
rust_targetdir = meson.current_build_dir() / 'cargo'
rust_lib = custom_target('rust_core',
  output: 'librust_core.a',
  depend_files: rust_core_rs_files,
  command: [
    'sh', '-c',
    'cargo build --manifest-path "$1" --release --target-dir "$2" && cp "$2/release/librust_core.a" "$3"',
    'cargo', rust_manifest, rust_targetdir, '@OUTPUT@',
  ],
  build_by_default: true,
  console: true,
)

if core_src_list.length() == 0
  error('No core .cpp files found in src/')
endif

core_src_list += meson.source_root() / 'src/socketengines' / (socket_engine + '.cpp')

inspircd = executable('inspircd', core_src_list,
  include_directories: inc,
  dependencies: [fmt_dep, dl_dep, pthread_dep],
  link_args: ['-rdynamic', '-Wl,--whole-archive', rust_lib.full_path(), '-Wl,--no-whole-archive'],
  export_dynamic: true,
  install: true,
  install_dir: get_option('bindir')
)

# ── Core modules (subdirectories) ───────────────────────
coremod_dirs = run_command(
  'find', meson.source_root() / 'src/coremods',
  '-mindepth', '1', '-maxdepth', '1', '-type', 'd',
  check: false
).stdout().strip().split('\n')

foreach cm_dir : coremod_dirs
  if cm_dir == ''
    continue
  endif
  cm_name = cm_dir.split('/')[-1]
  cm_srcs = run_command(
    'find', cm_dir, '-name', '*.cpp',
    check: false
  ).stdout().strip().split('\n')
  if cm_srcs.length() > 0 and cm_srcs[0] != ''
    if cm_name == 'core_info'
      # ADMIN command FFI lives in the Rust staticlib; link it into this module.
      shared_module(cm_name, cm_srcs,
        name_prefix: '',
        include_directories: inc,
        cpp_args: ['-DMODNAME="' + cm_name + '"'],
        link_args: ['-Wl,--whole-archive', rust_lib.full_path(), '-Wl,--no-whole-archive'],
        install: true,
        install_dir: 'modules'
      )
    else
      shared_module(cm_name, cm_srcs,
        name_prefix: '',
        include_directories: inc,
        cpp_args: ['-DMODNAME="' + cm_name + '"'],
        install: true,
        install_dir: 'modules'
      )
    endif
  endif
endforeach

# ── Core modules (single .cpp files directly in coremods) ─
coremod_standalone = run_command(
  'find', meson.source_root() / 'src/coremods',
  '-maxdepth', '1', '-name', '*.cpp',
  check: false
).stdout().strip().split('\n')

foreach cm_file : coremod_standalone
  if cm_file == ''
    continue
  endif
  cm_name = cm_file.split('/')[-1].split('.')[0]

  shared_module(cm_name, cm_file,
    name_prefix: '',
    include_directories: inc,
    cpp_args: ['-DMODNAME="' + cm_name + '"'],
    install: true,
    install_dir: 'modules'
  )
endforeach

# ── External modules (src/modules/) ─────────────────────
ext_mod_files = files(
  'src/modules/m_abbreviation.cpp',
  'src/modules/m_account.cpp',
  'src/modules/m_alias.cpp',
  'src/modules/m_allowinvite.cpp',
  'src/modules/m_alltime.cpp',
  'src/modules/m_anticaps.cpp',
  'src/modules/m_argon2.cpp',
  'src/modules/m_auditorium.cpp',
  'src/modules/m_autoop.cpp',
  'src/modules/m_banexception.cpp',
  'src/modules/m_banredirect.cpp',
  'src/modules/m_bcrypt.cpp',
  'src/modules/m_blockamsg.cpp',
  'src/modules/m_blockcolor.cpp',
  'src/modules/m_botmode.cpp',
  'src/modules/m_callerid.cpp',
  'src/modules/m_cap.cpp',
  'src/modules/m_cban.cpp',
  'src/modules/m_chancreate.cpp',
  'src/modules/m_chanfilter.cpp',
  'src/modules/m_chanhistory.cpp',
  'src/modules/m_chanlog.cpp',
  'src/modules/m_channames.cpp',
  'src/modules/m_channelban.cpp',
  'src/modules/m_check.cpp',
  'src/modules/m_chghost.cpp',
  'src/modules/m_chgident.cpp',
  'src/modules/m_chgname.cpp',
  'src/modules/m_classban.cpp',
  'src/modules/m_clearchan.cpp',
  'src/modules/m_cloak.cpp',
  'src/modules/m_cloak_md5.cpp',
  'src/modules/m_cloak_sha256.cpp',
  'src/modules/m_cloak_static.cpp',
  'src/modules/m_cloak_user.cpp',
  'src/modules/m_codepage.cpp',
  'src/modules/m_commonchans.cpp',
  'src/modules/m_connectban.cpp',
  'src/modules/m_connflood.cpp',
  'src/modules/m_conn_join.cpp',
  'src/modules/m_conn_umodes.cpp',
  'src/modules/m_conn_waitpong.cpp',
  'src/modules/m_customprefix.cpp',
  'src/modules/m_customtitle.cpp',
  'src/modules/m_cycle.cpp',
  'src/modules/m_dccallow.cpp',
  'src/modules/m_deaf.cpp',
  'src/modules/m_delayjoin.cpp',
  'src/modules/m_delaymsg.cpp',
  'src/modules/m_denychans.cpp',
  'src/modules/m_disable.cpp',
  'src/modules/m_dnsbl.cpp',
  'src/modules/m_exemptchanops.cpp',
  'src/modules/m_filter.cpp',
  'src/modules/m_gateway.cpp',
  'src/modules/m_geoban.cpp',
  'src/modules/m_geoclass.cpp',
  'src/modules/m_geo_maxmind.cpp',
  'src/modules/m_globalload.cpp',
  'src/modules/m_globops.cpp',
  'src/modules/m_haproxy.cpp',
  'src/modules/m_help.cpp',
  'src/modules/m_helpmode.cpp',
  'src/modules/m_hidechans.cpp',
  'src/modules/m_hidelist.cpp',
  'src/modules/m_hidemode.cpp',
  'src/modules/m_hideoper.cpp',
  'src/modules/m_hostcycle.cpp',
  'src/modules/m_httpd_acl.cpp',
  'src/modules/m_httpd_config.cpp',
  'src/modules/m_httpd.cpp',
  'src/modules/m_httpd_stats.cpp',
  'src/modules/m_ident.cpp',
  'src/modules/m_inviteexception.cpp',
  'src/modules/m_ircv3_accounttag.cpp',
  'src/modules/m_ircv3_batch.cpp',
  'src/modules/m_ircv3_capnotify.cpp',
  'src/modules/m_ircv3_chghost.cpp',
  'src/modules/m_ircv3.cpp',
  'src/modules/m_ircv3_ctctags.cpp',
  'src/modules/m_ircv3_echomessage.cpp',
  'src/modules/m_ircv3_invitenotify.cpp',
  'src/modules/m_ircv3_labeledresponse.cpp',
  'src/modules/m_ircv3_msgid.cpp',
  'src/modules/m_ircv3_servertime.cpp',
  'src/modules/m_ircv3_sts.cpp',
  'src/modules/m_ircv3_utf8only.cpp',
  'src/modules/m_joinflood.cpp',
  'src/modules/m_kicknorejoin.cpp',
  'src/modules/m_knock.cpp',
  'src/modules/m_ldapauth.cpp',
  'src/modules/m_ldap.cpp',
  'src/modules/m_ldapoper.cpp',
  'src/modules/m_log_sql.cpp',
  'src/modules/m_log_syslog.cpp',
  'src/modules/m_maphide.cpp',
  'src/modules/m_md5.cpp',
  'src/modules/m_messageflood.cpp',
  'src/modules/m_monitor.cpp',
  'src/modules/m_multiprefix.cpp',
  'src/modules/m_muteban.cpp',
  'src/modules/m_mysql.cpp',
  'src/modules/m_namedmodes.cpp',
  'src/modules/m_nickflood.cpp',
  'src/modules/m_nicklock.cpp',
  'src/modules/m_noctcp.cpp',
  'src/modules/m_nokicks.cpp',
  'src/modules/m_nonicks.cpp',
  'src/modules/m_nonotice.cpp',
  'src/modules/m_ojoin.cpp',
  'src/modules/m_operchans.cpp',
  'src/modules/m_operjoin.cpp',
  'src/modules/m_operlevels.cpp',
  'src/modules/m_operlog.cpp',
  'src/modules/m_opermodes.cpp',
  'src/modules/m_opermotd.cpp',
  'src/modules/m_operprefix.cpp',
  'src/modules/m_opmoderated.cpp',
  'src/modules/m_override.cpp',
  'src/modules/m_passforward.cpp',
  'src/modules/m_password_hash.cpp',
  'src/modules/m_pbkdf2.cpp',
  'src/modules/m_permchannels.cpp',
  'src/modules/m_pgsql.cpp',
  'src/modules/m_randquote.cpp',
  'src/modules/m_realnameban.cpp',
  'src/modules/m_redirect.cpp',
  'src/modules/m_regex_glob.cpp',
  'src/modules/m_regex_pcre2.cpp',
  'src/modules/m_regex_posix.cpp',
  'src/modules/m_regex_re2.cpp',
  'src/modules/m_regex_stdlib.cpp',
  'src/modules/m_remove.cpp',
  'src/modules/m_repeat.cpp',
  'src/modules/m_restrictchans.cpp',
  'src/modules/m_restrictmsg.cpp',
  'src/modules/m_rline.cpp',
  'src/modules/m_rmode.cpp',
  'src/modules/m_sajoin.cpp',
  'src/modules/m_sakick.cpp',
  'src/modules/m_samode.cpp',
  'src/modules/m_sanick.cpp',
  'src/modules/m_sapart.cpp',
  'src/modules/m_saquit.cpp',
  'src/modules/m_sasl.cpp',
  'src/modules/m_satopic.cpp',
  'src/modules/m_securelist.cpp',
  'src/modules/m_seenicks.cpp',
  'src/modules/m_serverban.cpp',
  'src/modules/m_services.cpp',
  'src/modules/m_sethost.cpp',
  'src/modules/m_setident.cpp',
  'src/modules/m_setidle.cpp',
  'src/modules/m_setname.cpp',
  'src/modules/m_sha1.cpp',
  'src/modules/m_sha2.cpp',
  'src/modules/m_sharebans.cpp',
  'src/modules/m_showfile.cpp',
  'src/modules/m_showwhois.cpp',
  'src/modules/m_shun.cpp',
  'src/modules/m_silence.cpp',
  'src/modules/m_sqlauth.cpp',
  'src/modules/m_sqlite3.cpp',
  'src/modules/m_sqloper.cpp',
  'src/modules/m_ssl_gnutls.cpp',
  'src/modules/m_sslinfo.cpp',
  'src/modules/m_sslmodes.cpp',
  'src/modules/m_ssl_openssl.cpp',
  'src/modules/m_sslrehashsignal.cpp',
  'src/modules/m_starttls.cpp',
  'src/modules/m_stripcolor.cpp',
  'src/modules/m_swhois.cpp',
  'src/modules/m_timedbans.cpp',
  'src/modules/m_tline.cpp',
  'src/modules/m_uhnames.cpp',
  'src/modules/m_uninvite.cpp',
  'src/modules/m_vhost.cpp',
  'src/modules/m_watch.cpp',
  'src/modules/m_websocket.cpp',
  'src/modules/m_xline_db.cpp',
)

foreach src_file : ext_mod_files
  if fs.name(src_file) == ''
    continue
  endif
  mod_name = fs.name(src_file).split('/')[-1].split('.')[0]
  mod_deps = []
  mod_includes = inc
  if mod_name == 'm_gateway'
    if not mongoc_dep.found()
      warning('MongoDB libraries not found — skipping m_gateway')
      continue
    endif
    mod_deps = [mongoc_dep]
  endif  
  if mod_name == 'm_mysql'
    mod_deps = [dependency('mariadb', required: true)]
  endif
  if mod_name == 'm_sqlite3'
    mod_deps = [sqlite3_dep]
  endif
  if mod_name == 'm_pgsql'
    mod_deps = []
    mod_includes = [inc, inc_pgsql]
  endif
  if mod_name == 'm_argon2'
    mod_deps = [argon2_dep]
  endif
  if mod_name == 'm_ssl_openssl'
      mod_deps = [ssl_dep]
  endif
  if mod_name == 'm_ssl_gnutls'
      mod_deps = [gnutls_dep]
  endif
  shared_module(mod_name, src_file,
    include_directories: mod_includes,
    name_prefix: '',
    cpp_args: ['-DMODNAME="' + mod_name + '"'],
    dependencies: mod_deps,
    install: true,
    install_dir: 'modules'
  )
endforeach
