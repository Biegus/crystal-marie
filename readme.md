# Crystal-Marie compilier

Compiles crysta-marie code to marie assembly code.    

What's marie assembly? Assembly for theoretical very simple cpu.  
The only marie assembly intepreter i am aware of  is avaiable here (sadly online) 
https://marie.js.org/  
I am not aware of existense of any other interpreter.

What's crystal marie? 
A little higher  version of marie made by me.
For crystal-marie doc see *Crystal Marie Doc* header

## Usage 

download release https://github.com/Biegus/crystal-marie/releases/tag/0.9  

(if you want to build binaries yourself look at *Builing Compiler*)

```shell
crystal-marie file.crmarie     
```

### Optional arguments

**`-l libfile.crmarie`**  
Include lib (you can include multiple files, order matters)  
note: in case of error in lib file, the line of error will be incorrect (something negative) 

note: Its recommended to include std.crmarie as lib file (its in main repo folder or in any release)   
Without std.crmarie the stack_function feature won't work as it requires special functions defined in crystal-marie language

```shell
crystal-marie file.crmarie -l std.crmarie
```

**`-o output_name.marie`** 
specify output file name (by default it's a.marie) 


**`-s`**
instead of writing to file, output to stdout


## Building compiler
you only need cargo installed  

```shell
cargo build --release 
```

## Crystal Marie Doc
**This docs are going to assume you included std.crmarie as lib**   

### Heavy commented examples
to see outputs of examples see examples/from_readme/output   
note: the output will include all std functions, even the ones that are not used


*output=input program*
```crystal-marie
*                   // * star means end of global variable list 
function main       // every program stars with main function
x=0                 // local variables  (those are NOT args)
*                   // end of local variables list
{                   // block codes and functions start and end with braces
    x=input()       // calls a function and puts return value into x
    output(x)       // calls a function with local variable
}                   // end of function

```

*truth machine*
```crystl-marie 
*
function main
x=0                             // local variable that will hold the input
*
{
    x=input()      
    .if (EQ x 0)                //non functions operand start with "." if x==0
    {
        output(0)               // this code happens if x==0
    }
    .else
    {
        .flag(beg)              // defines labels, the name is local to the function
        output(1)            
        %jump main_flag_beg     // % does inline marie assembly
    }
}

```

*recursive fib*
```crystal-marie
*
//resurive functions has to be marked as "stack_function" 
//they will use stack instead of simple function assignments and jump
//stack function can be called only from
//other stack functions or from main
stack_function fib x  //x is the function argument
x_prev_1=0  
x_prev_2=0
x_res_1=0
x_res_2=0
temp=0
*
{
    .if(EQ x 0)
    {
        .ret(0)                 // returns given value and exits function
    }
    .noelse // this can be written instead of else clause

    .if(EQ x 1)
    {
        .ret(1)
    }
    .noelse

    x_prev_1 = sub(x 1)
    x_prev_2 = sub(x 2)

    x_res_1 = fib(x_prev_1)
    x_res_2 = fib(x_prev_2)

    temp = add(x_res_1 x_res_2)  //notice no commas in function call
    .ret(temp)

    //having to have temp can be avoided by writing
    //return = add(x_res_1 x_res_2)  
    //be careful assignment to the return variable is guaranteed
    //to work only if next line ends function  

}
function main
x=0
temp=0
*
{
    x=input()
    temp=fib(x)
    output(temp)

}

```

### Specifics 




#### Functions

All functions can only be called from the functions defined above them. 
Functions split into two categories depending whether they can call themselves


- Normal functions  
They can't "call themselves" in direct 
The calls are carried only with variable assignent and jump. 

- Stack function  
They used the stack which allows them to call themselves in direct way  
Stack function can only be called from other stack functions or from main.   
You should only use them when you are gonna call stack functions or need recursion.   
For recursion keep in mind program will not optimize for not putting not used locals on the stack.   
Keep amount of local variables low
 

Every program must define (non stack) main function (unless its a lib file)  

Every function has an argument list and local list  
Arguments are written right after the function name        
Local variables are written one per line below
The list of local variables is ended with a star  
Content of function is in braces

*normal function defintiion*
```crystal-marie
function my_fnc arg_1 arg_2 
local_1=0
local_2=3 
*
{
    //add code here
    .ret(3) // optional return some result
    //if you don't return specific value 
    //the acc value will be returned which may be garbage
}

```


*stack function defintiion*
```crystal-marie
stack_function my_fnc arg_1 arg_2 
local_1=0
local_2=3 
*
{

    //add code here
    //can call stack functions and yourself
    .ret(3) // optional return some result 

}

```


#### Copy operation

with copy operation you can assign variables to:

`1 ` literal *positive* numbers   
`x` variables   
`*x` (deref) values under addresses   
`&x` addresses of variables

```
x:=0 // sets x to 0
x:=y // sets x to y
x:=&y // sets x to address of y 
x:=*y // sets x to the value under they address
```

#### Global variables
they have to be defined at the beginning of the file  
Line after last global variable has to be `*`   

`name = start_val`   
where start_val has to be positive number


```crystal-marie
x=0
y=0
*
function main()
{
    //code
}
```

#### Function calls 

functions can be called by their name with args  
written **without commans** in parenthesis. 

their return value maybe be caputured with `=` to local or global variable.  
Right after the call it will be also in "return" variable 


```
my_func(arg_1 arg_2 arg_3)      //call function ignoring the return value

x = my_func(arg_1 arg_2 arg_3)  //call function putting the return value into x

my_func(arg_1 arg_2 arg_3) // also the same
x:=return

```

args can take anything copy operator can take so,  

`1 ` literal *positive* numbers   
`x` variables   
`*x` (deref) values under addresses   
`&x` addresses of variables


Compiler defines no functions in itself.  
The ones we use come from std.crmarie file.   



**Stack functions requirments**   
If you use stack_function the compiler expects   
you to define push pop and stack_return functions   
Those are defined in std as well, so if you include it you are fine.  
The assumptions compiles makes about them:        
push(x) -> pushes x on the stack, returns nothing   
pop() -> gets last pushed value and removes it from the stack  
stack_return(x) -> pops address of the stack and jumps to it  



#### .if 

`EQ` x==y   
`LESS` x<y   
`MORE` x>y 

if condition has two forms: with else or without

ifs can take any arguments acceptable in function call as elements  so

`1 ` literal *positive* numbers   
`x` variables   
`*x` (deref) values under addresses   
`&x` addresses of variables



*with else*
```
//...
.if(EQ x y)
{
    //x==y
}
.else
{
    //x !=y
}
//...

```

*without else*
```
//...
.if(MORE x 0)
{
    //x>0
}
.noelse  // you have to add this and can't just have .if alone
//...

``` 

#### .ret
return from current function with value
```
//...
function my_func  x
temp=0
{
    //do some epic calculations and puts result in temp
    .ret(temp)  
    %halt // this won't  execute
}
//...

```

#### Inline marie assembly

starting with `%` will make the line compile inline. 

Keep in mind you have to know some things about how compile works to use this. Code inside % won't be checked for correctness (for examples whether it uses a variable that exists) 

You have to use full qualified names in inline  
since there's no abstraction there.

- local `x` defined in `my_func` turns into `var_my_func_x`  
- global `y` turns into `var_y` 
- flag `z` defined in `my_func` turns into `flag_my_func_z`   
- label of function `my_func` turns into `function_my_func`  


main use of inline assembly out of std is direct jumps since there's no support for them directly in crystal-marie

you shouldn't try to access `if` labels from inline assembly. 
They will have number appended that probably will be probably 
be equal to number of if-like structures above.

you can assume that right after the function call acc will be equal to return value.


#### Std Functions

functions defined in std 

|  fnc | what it does|
| --- | ----|
| `output(x)` | outputs x  | 
| `input` | returns input |
|`add(x y)`| x+y|
|`sub(x y)` | x-y|
| `copy_into(x y)` | (*y)=x
| `halt` | ends program
| `push(x)`| stack.push(x) used by stack functions so use if you know what you are doing|
| `pop()` | stack.pop()pused by stack functions so use if you know what you are doing |
| `stack_return` | don't use this manually | 




