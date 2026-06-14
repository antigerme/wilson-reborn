##Windows:
##https://meng-milling.dk/jc_reborn.msi
Não sou muito de confiar em binário windows distribuídos na Internet então criei...


## Como criei 

# Clone o repositório
git clone https://github.com/jno6809/jc_reborn

# Aplique esta pequena correção
sed -i 's/localTime = localtime(&tv\.tv_sec);/time_t t = tv.tv_sec;\n    localTime = localtime(\&t);/g' utils.c

# Compile o binário Linux
dnf install SDL2-devel 
make -f Makefile.linux

# Compile o executável Windows
dnf install mingw64-gcc mingw64-gcc-c++ mingw64-SDL2
make -f Makefile.MinGW CC=x86_64-w64-mingw32-gcc CFLAGS="-I/usr/x86_64-w64-mingw32/sys-root/mingw/include/ -Wall --std=c99" LDFLAGS="-L/usr/x86_64-w64-mingw32/sys-root/mingw/lib/ -mwindows" LDLIBS="-lmingw32 -lSDL2main -lSDL2"

# Dist
Crie uma pasta chamada dist e copie para dentro dela:
Os recursos originais da aplicação como RESOURCE.001, RESOURCE.MAP e os *.wav
Copie o binário para linux e o binário windows (jc_reborn.exe mas renomeando para jc_reborn.scr).
Para windows também copie o /usr/x86_64-w64-mingw32/sys-root/mingw/bin/SDL*.dll

Obs.: o gdrive tava de arte com o zip e por isso o danado tá com a senha "felicio"
