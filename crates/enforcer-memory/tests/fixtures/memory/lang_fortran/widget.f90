module shapes
  implicit none
contains
  function area(r) result(a)
    real :: r, a
    a = 3.14 * r * r
  end function area

  subroutine greet(name)
    character(len=*) :: name
    call helper(name)
  end subroutine greet

  subroutine helper(name)
    character(len=*) :: name
    print *, name
  end subroutine helper
end module shapes
