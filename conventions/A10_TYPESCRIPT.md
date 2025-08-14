## General Typescript/Javascript Style Guidelines
- always use snake_case names for variables and function names, even in typescript and javascript
- Use the Early Exit and Return Early patterns 
- Use Guard Clauses conditional where needed  
- Keep the function code as flat as possible, minimizing indented logic wherever possible 
- in try catch blocks, keep the block very short, having only statements that could fail inside the block.  
- when using try catch blocks, keep try block on single line where possible. example:
    ```
    - const propertya = "abc";  
    - let   result:string|null = null;
    - try   { result = await someasyncfunction(propertya); }
    - catch { rej(); return; }
    ```
- use awaits where possible, vs promise chaining. 
