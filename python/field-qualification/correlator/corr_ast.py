# Copyright 2024 Two Six Technologies.

import ast

if __name__ == "__main__":
    args = ast.arguments(
        args=[
            ast.arg(arg="quic.dcil"),
            ast.arg(arg="quic.dcid"),
        ]
    )

    body = [
        ast.Return(
            value=ast.Call(
                func=ast.Name(id="len", ctx=ast.Load()),
                args=[ast.Name(id="quic.dcid", ctx=ast.Load())],
                keywords=[],
            )
        )
    ]

    function_def = ast.FunctionDef(
        name="quic_dcil_generator", args=args, body=body, decorator_list=[]
    )

    print(ast.dump(function_def))
