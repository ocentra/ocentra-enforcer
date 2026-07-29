function! Greet(name)
  echo "hello " . a:name
  call Helper(a:name)
endfunction

function! Helper(name)
  if a:name == ""
    echo "empty"
  endif
endfunction

let g:foo = 1
