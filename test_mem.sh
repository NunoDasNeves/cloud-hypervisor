#!/bin/bash
set -x

VM_MEM_GIB=4
CH_MEM_ARG="${VM_MEM_GIB}G"
VMM_MEM_ARG=$(echo "$VM_MEM_GIB*1024*1024" | bc -l)

CH_PATH="$HOME/cloud-hypervisor"
CH_BIN="$CH_PATH/target/release/cloud-hypervisor"
CH_BIN_PATCHED="$CH_PATH/target/release/cloud-hypervisor-patched"
VFIO_ARG="--device path=/sys/bus/pci/devices/0000:01:00.1"

VMM_MEMORY_BIN="$HOME/vmm_memory/target/release/vmm_memory"

KiB_to_GiB () {
	echo "$1/1024/1024" | bc -l
}

get_proc_meminfo_line_GiB () {
	KiB=$(cat /proc/meminfo | grep "^$1:" | sed -E 's#.*\s([0-9]+)\s.*#\1#')
	echo $(KiB_to_GiB "$KiB")
}
printf "MemTotal: %.2f GiB\n\n" "$(get_proc_meminfo_line_GiB "MemTotal")" > summary.txt
printf "VM size: %s\n" $CH_MEM_ARG >> summary.txt

printf "%-10s%-10s%-10s%-15s%-15s%-15s%-15s\n" "vfio" "shared" "patched" "MemAvailable" "Active(anon)" "AnonPages" "Mapped" "Shmem" >> summary.txt
printf "          (baseline)          %-2.2f GiB      %-2.2f GiB      %-2.2f GiB      %-2.2f GiB      %-2.2f GiB      \n" "$(get_proc_meminfo_line_GiB 'MemAvailable')" "$(get_proc_meminfo_line_GiB 'Active(anon)')" "$(get_proc_meminfo_line_GiB 'AnonPages')" "$(get_proc_meminfo_line_GiB 'Mapped')" "$(get_proc_meminfo_line_GiB 'Shmem')" >> summary.txt

for vfio in "no" "yes"
do
	for shared in "no" "yes"
	do
		for patched in "no" "yes"
		do
			vfio_arg=""
			shared_on="off"
			bin="$CH_BIN"
			if [ "$vfio" = "yes" ]
			then
				vfio_arg="$VFIO_ARG"
			fi
			if [ "$shared" = "yes" ]
			then
				shared_on="on"
			fi
			if [ "$patched" = "yes" ]
			then
				bin="$CH_BIN_PATCHED"
			fi
            LOG_SUFFIX="_vfio_${vfio}_shared_${shared}_patched_${patched}"
			sudo $bin --kernel $CH_PATH/hypervisor-fw --disk path=$CH_PATH/focal-server-cloudimg-amd64.raw --cpus boot=4 --memory size="$CH_MEM_ARG",shared="$shared_on" --net "tap=,mac=,ip=,mask=" ${vfio_arg} -v --log-file "ch${LOG_SUFFIX}.log" &
			SUDO_PID=$!
			sleep 1
			CH_PID=$(sudo pgrep -P $SUDO_PID)
			sleep 15

            PROC_MEMINFO_OUTPUT="$(sudo cat /proc/meminfo)"
            VMM_MEM_OUTPUT="$(sudo ${VMM_MEMORY_BIN} ${CH_PID} --size=${VMM_MEM_ARG})"
            echo "$VMM_MEM_OUTPUT" > "vmm_memory${LOG_SUFFIX}.txt"
            echo "$PROC_MEMINFO_OUTPUT" > "proc_meminfo${LOG_SUFFIX}.txt"
			KIB_OVERHEAD=$(echo "$VMM_MEM_OUTPUT" | grep "Total Overhead" | sed -E 's#.*\s([0-9]+)\s.*#\1#')
			printf "%-10s%-10s%-10s%-2.2f GiB      %-2.2f GiB      %-2.2f GiB      %-2.2f GiB      %-2.2f GiB      \n" "$vfio" "$shared" "$patched" "$(get_proc_meminfo_line_GiB 'MemAvailable')" "$(get_proc_meminfo_line_GiB 'Active(anon)')" "$(get_proc_meminfo_line_GiB 'AnonPages')" "$(get_proc_meminfo_line_GiB 'Mapped')" "$(get_proc_meminfo_line_GiB 'Shmem')" >> summary.txt
			sudo pkill cloud
			sleep 2
		done
	done
done

