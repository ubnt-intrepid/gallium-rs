" :set exrc
" :set secure

augroup rust_config
  autocmd!
  autocmd FileType rust set makeprg=./scripts/cargo.sh
augroup END
